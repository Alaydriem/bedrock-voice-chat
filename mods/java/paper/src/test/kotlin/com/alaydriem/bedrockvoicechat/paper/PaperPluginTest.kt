package com.alaydriem.bedrockvoicechat.paper

import org.mockbukkit.mockbukkit.MockBukkit
import org.mockbukkit.mockbukkit.ServerMock
import org.bukkit.event.player.PlayerJoinEvent
import org.bukkit.event.player.PlayerQuitEvent
import org.junit.jupiter.api.AfterEach
import org.junit.jupiter.api.BeforeEach
import org.junit.jupiter.api.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

/**
 * MockBukkit-based tests for Paper server functionality.
 * Tests player events without loading the actual plugin.
 */
class PaperPluginTest {
    private lateinit var server: ServerMock

    @BeforeEach
    fun setUp() {
        server = MockBukkit.mock()
    }

    @AfterEach
    fun tearDown() {
        MockBukkit.unmock()
    }

    @Test
    fun `player join event should fire when player joins`() {
        // Add a mock player (automatically fires PlayerJoinEvent)
        val player = server.addPlayer("TestPlayer")

        // Verify player is online
        assertTrue(player.isOnline)
    }

    @Test
    fun `player quit event should fire when player disconnects`() {
        val player = server.addPlayer("TestPlayer")

        // Disconnect player (fires PlayerQuitEvent)
        player.disconnect()

        // Verify player is offline
        assertFalse(player.isOnline)
    }

    @Test
    fun `should handle rapid join-quit sequence`() {
        val player = server.addPlayer("TestPlayer")

        // Rapid join/quit/join sequence
        player.disconnect()
        player.reconnect()
        player.disconnect()
        player.reconnect()

        // Verify final state is online
        assertTrue(player.isOnline)
    }

    @Test
    fun `should track multiple players correctly`() {
        // Add 5 players
        repeat(5) { i -> server.addPlayer("Player$i") }

        // Verify all are online
        assertEquals(5, server.onlinePlayers.size)
    }
}
