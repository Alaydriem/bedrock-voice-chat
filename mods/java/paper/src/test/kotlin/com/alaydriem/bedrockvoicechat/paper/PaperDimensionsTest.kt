package com.alaydriem.bedrockvoicechat.paper

import org.bukkit.World
import org.bukkit.WorldCreator
import org.junit.jupiter.api.AfterEach
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.BeforeEach
import org.junit.jupiter.api.Test
import org.mockbukkit.mockbukkit.MockBukkit
import org.mockbukkit.mockbukkit.ServerMock

class PaperDimensionsTest {
    private lateinit var server: ServerMock

    @BeforeEach
    fun setUp() {
        server = MockBukkit.mock()
    }

    @AfterEach
    fun tearDown() {
        MockBukkit.unmock()
    }

    private fun world(name: String, environment: World.Environment): World =
        server.createWorld(WorldCreator(name).environment(environment))!!

    // Positions already sent the world name; the jukebox and the SVC bridge sent
    // "overworld", which put a custom world's audio beside Overworld listeners.
    @Test
    fun `a custom world is named, never overworld`() {
        assertEquals("aether", PaperDimensions.of(world("aether", World.Environment.CUSTOM)).toApiString())
    }

    @Test
    fun `vanilla environments map to vanilla dimensions`() {
        assertEquals("nether", PaperDimensions.of(world("world_nether", World.Environment.NETHER)).toApiString())
        assertEquals("the_end", PaperDimensions.of(world("world_the_end", World.Environment.THE_END)).toApiString())
        assertEquals("overworld", PaperDimensions.of(world("lobby", World.Environment.NORMAL)).toApiString())
    }

    @Test
    fun `no world is overworld`() {
        assertEquals("overworld", PaperDimensions.of(null).toApiString())
    }
}
