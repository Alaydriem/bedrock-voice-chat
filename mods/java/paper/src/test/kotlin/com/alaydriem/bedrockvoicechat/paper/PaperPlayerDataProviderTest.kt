package com.alaydriem.bedrockvoicechat.paper

import org.mockbukkit.mockbukkit.MockBukkit
import org.mockbukkit.mockbukkit.ServerMock
import com.alaydriem.bedrockvoicechat.dto.GameType
import org.junit.jupiter.api.AfterEach
import org.junit.jupiter.api.BeforeEach
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertTrue

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
        provider.server = server
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

    /**
     * The two identity forms are not interchangeable.
     *
     * Everything the BVC server indexes a player by is keyed `game:gamertag`, and a
     * bare gamertag matches none of it. It does not fail either — it answers no, for
     * every player, which is how the SVC bridge came to mark every BVC player as
     * disconnected and to suppress double audio for nobody.
     */
    @Test
    fun `the membership key carries the game prefix and the canonical name does not`() {
        val player = server.addPlayer("TestPlayer")

        assertEquals("TestPlayer", provider.resolveCanonicalName(player))
        assertEquals(listOf("minecraft:TestPlayer"), provider.resolveMembershipKeys(player))
    }

    /**
     * A linked Bedrock player answers to two names and we cannot tell which one their
     * BVC client registered under: the mod sends their Java account name while the
     * client authenticates against Xbox Live. Offering only one of them misses that
     * player whichever is chosen.
     */
    @Test
    fun `a linked player offers both their java name and their gamertag`() {
        val provider = PaperPlayerDataProvider(xboxGamertagOf = { "XboxTag" })
        provider.server = server
        val player = server.addPlayer("JavaName")

        val keys = provider.resolveMembershipKeys(player)

        assertTrue(keys.contains("minecraft:JavaName"), "java name missing from $keys")
        assertTrue(keys.contains("minecraft:XboxTag"), "gamertag missing from $keys")
    }

    /**
     * A Floodgate-prefixed player is already known by their gamertag alone, so the
     * prefixed Java username is not a second identity — it is the same one wearing a
     * prefix, and nothing registers under it.
     */
    @Test
    fun `a prefixed bedrock player offers only their gamertag`() {
        val provider = PaperPlayerDataProvider(xboxGamertagOf = { "XboxTag" })
        provider.server = server
        val player = server.addPlayer(".XboxTag")

        assertEquals(listOf("minecraft:XboxTag"), provider.resolveMembershipKeys(player))
    }

    /**
     * One name in, one key out. A player whose two names agree must not be offered
     * twice, or every lookup for them costs two.
     */
    /**
     * The name a frame carries came out of [PaperPlayerDataProvider.resolveCanonicalName],
     * so it must go back in. Looking a speaker up by their raw profile name is a
     * different question, and answering it wrongly is silent: the speaker falls back
     * to a fixed position, which sounds close enough that only the missing talking
     * indicator gives it away.
     */
    @Test
    fun `a canonical name finds the body it was taken from`() {
        val player = server.addPlayer("TestPlayer")

        val found = provider.findByIdentity(provider.resolveCanonicalName(player))

        assertEquals(player.uniqueId, found?.uniqueId)
    }

    // The Floodgate case, where the profile name and the canonical name genuinely
    // differ. Matching the profile name finds nobody for exactly the players the
    // bridge exists to carry.
    @Test
    fun `a prefixed bedrock player is found by their gamertag`() {
        val provider = PaperPlayerDataProvider(xboxGamertagOf = { "XboxTag" })
        provider.server = server
        val player = server.addPlayer(".XboxTag")

        assertEquals(player.uniqueId, provider.findByIdentity("XboxTag")?.uniqueId)
    }

    // A linked player is named by their Java account here and may be named by their
    // gamertag on the far side, so both have to come back to the same body.
    @Test
    fun `a linked player is found by either of their names`() {
        val provider = PaperPlayerDataProvider(xboxGamertagOf = { "XboxTag" })
        provider.server = server
        val player = server.addPlayer("JavaName")

        assertEquals(player.uniqueId, provider.findByIdentity("JavaName")?.uniqueId)
        assertEquals(player.uniqueId, provider.findByIdentity("XboxTag")?.uniqueId)
    }

    // A speaker on another server has no body here, and inventing one would attach
    // their audio to a stranger.
    @Test
    fun `a name nobody answers to finds no body`() {
        server.addPlayer("TestPlayer")

        assertNull(provider.findByIdentity("SomeoneElse"))
    }

    @Test
    fun `a player whose names agree is offered once`() {
        val provider = PaperPlayerDataProvider(xboxGamertagOf = { "SameName" })
        provider.server = server
        val player = server.addPlayer("SameName")

        assertEquals(listOf("minecraft:SameName"), provider.resolveMembershipKeys(player))
    }
}
