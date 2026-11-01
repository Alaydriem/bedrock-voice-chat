package com.alaydriem.bedrockvoicechat.paper.svc

import de.maxhenkel.voicechat.api.VoicechatServerApi
import org.bukkit.World
import org.bukkit.WorldCreator
import org.junit.jupiter.api.AfterEach
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.BeforeEach
import org.junit.jupiter.api.Test
import org.mockbukkit.mockbukkit.MockBukkit
import org.mockbukkit.mockbukkit.ServerMock
import java.lang.reflect.Proxy
import java.util.UUID

class PaperSvcChannelFactoryTest {
    private lateinit var server: ServerMock
    private lateinit var factory: PaperSvcChannelFactory

    // The world each locational open was asked for, recorded at `fromServerLevel`.
    private val opened = mutableListOf<World>()
    private val hashed = mutableListOf<String>()

    private val api = Proxy.newProxyInstance(
        VoicechatServerApi::class.java.classLoader,
        arrayOf(VoicechatServerApi::class.java)
    ) { _, method, args ->
        if (method.name == "fromServerLevel") {
            opened += args[0] as World
        }
        null
    } as VoicechatServerApi

    // Stands in for the relay codec: vanilla names pass through, anything else is "hashed".
    private val wireForm: (String) -> String = { name ->
        hashed += name
        if (name in setOf("overworld", "nether", "the_end")) name else "custom:$name"
    }

    @BeforeEach
    fun setUp() {
        server = MockBukkit.mock()
        listOf(
            "world" to World.Environment.NORMAL,
            "world_nether" to World.Environment.NETHER,
            "world_the_end" to World.Environment.THE_END,
            "aether" to World.Environment.CUSTOM,
        ).forEach { (name, environment) -> server.createWorld(WorldCreator(name).environment(environment)) }
        factory = PaperSvcChannelFactory(api, server, { null }, wireForm)
    }

    @AfterEach
    fun tearDown() {
        MockBukkit.unmock()
    }

    private fun open(dimension: String): World? {
        opened.clear()
        factory.locationalChannel(UUID.randomUUID(), dimension, 0.0, 64.0, 0.0)
        return opened.singleOrNull()
    }

    @Test
    fun `each vanilla dimension opens in the world of its environment`() {
        assertEquals(World.Environment.NORMAL, open("overworld")?.environment)
        assertEquals(World.Environment.NETHER, open("nether")?.environment)
        assertEquals(World.Environment.THE_END, open("the_end")?.environment)
    }

    @Test
    fun `a custom dimension opens in the world whose name hashes to it`() {
        assertEquals("aether", open("custom:aether")?.name)
    }

    @Test
    fun `a dimension this server does not have opens nothing`() {
        assertNull(open("death"))
        assertNull(open("custom:missing"))
    }

    // A failed open is retried on every frame, so a speaker in a dimension this server lacks
    // would otherwise cost one codec call per world per frame.
    @Test
    fun `each world is hashed once however often a missing dimension is asked for`() {
        repeat(3) { open("custom:missing") }

        assertEquals(hashed.distinct(), hashed)
    }
}
