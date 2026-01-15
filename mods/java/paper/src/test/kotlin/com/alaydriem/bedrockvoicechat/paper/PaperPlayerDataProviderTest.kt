package com.alaydriem.bedrockvoicechat.paper

import org.mockbukkit.mockbukkit.MockBukkit
import org.mockbukkit.mockbukkit.ServerMock
import com.alaydriem.bedrockvoicechat.dto.GameType
import org.junit.jupiter.api.AfterEach
import org.junit.jupiter.api.BeforeEach
import org.junit.jupiter.api.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

/**
 * Tests for PaperPlayerDataProvider using MockBukkit.
 */
class PaperPlayerDataProviderTest {
    private lateinit var server: ServerMock
    private lateinit var provider: PaperPlayerDataProvider

    @BeforeEach
    fun setUp() {
        server = MockBukkit.mock()
        provider = PaperPlayerDataProvider()
    }

    @AfterEach
    fun tearDown() {
        MockBukkit.unmock()
    }

    @Test
    fun `collectPlayers returns empty list when no players tracked`() {
        val players = provider.collectPlayers()
        assertTrue(players.isEmpty())
    }

    @Test
    fun `collectPlayers returns correct player data after addPlayer`() {
        // Create world and add player
        val world = server.addSimpleWorld("world")
        val player = server.addPlayer("TestPlayer")

        // Set position
        player.teleport(world.spawnLocation.add(100.0, 64.0, 200.0))

        // Add player to provider
        provider.addPlayer(player)

        val players = provider.collectPlayers()

        assertEquals(1, players.size)
        assertEquals("TestPlayer", players[0].name)
    }

    @Test
    fun `collectPlayers should return multiple players`() {
        val world = server.addSimpleWorld("world")
        val player1 = server.addPlayer("Player1")
        val player2 = server.addPlayer("Player2")

        // Set positions
        player1.teleport(world.spawnLocation.add(100.0, 64.0, 200.0))
        player2.teleport(world.spawnLocation.add(50.0, 70.0, 150.0))

        // Add players to provider
        provider.addPlayer(player1)
        provider.addPlayer(player2)

        val players = provider.collectPlayers()

        assertEquals(2, players.size)
        assertTrue(players.any { it.name == "Player1" })
        assertTrue(players.any { it.name == "Player2" })
    }

    @Test
    fun `removePlayer should remove player from tracking`() {
        val player = server.addPlayer("TestPlayer")

        provider.addPlayer(player)
        assertEquals(1, provider.collectPlayers().size)

        provider.removePlayer(player)
        assertEquals(0, provider.collectPlayers().size)
    }

    @Test
    fun `collectPlayers should filter out offline players`() {
        val player = server.addPlayer("TestPlayer")

        provider.addPlayer(player)
        assertEquals(1, provider.collectPlayers().size)

        // Disconnect player (makes isOnline return false)
        player.disconnect()

        // Should be filtered out by isOnline check
        assertEquals(0, provider.collectPlayers().size)
    }

    @Test
    fun `getGameType should return MINECRAFT`() {
        assertEquals(GameType.MINECRAFT, provider.getGameType())
    }
}
