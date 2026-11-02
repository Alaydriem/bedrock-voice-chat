package com.alaydriem.bedrockvoicechat.paper

import org.mockbukkit.mockbukkit.MockBukkit
import org.mockbukkit.mockbukkit.ServerMock
import org.mockbukkit.mockbukkit.entity.PlayerMock
import com.alaydriem.bedrockvoicechat.dto.GameType
import org.junit.jupiter.api.AfterEach
import org.junit.jupiter.api.BeforeEach
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertTrue
import java.util.UUID

/**
 * Tests for PaperPlayerDataProvider using MockBukkit.
 *
 * MockBukkit gives players random v4 UUIDs, which read as a linked player's Java UUID.
 * Every unlinked Floodgate player here is built with Floodgate's own UUID shape,
 * UUID(0, xuid), or the test exercises the linked branch without saying so.
 */
class PaperPlayerDataProviderTest {
    private lateinit var server: ServerMock
    private lateinit var provider: PaperPlayerDataProvider

    private val floodgate = UUID(0L, 2535428000000001L)
    private val floodgate2 = UUID(0L, 2535428000000002L)

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

    private fun providerWith(gamertags: Map<UUID, String>): PaperPlayerDataProvider {
        val p = PaperPlayerDataProvider(xboxGamertagOf = gamertags::get)
        p.server = server
        return p
    }

    private fun floodgatePlayer(javaName: String, uuid: UUID = floodgate): PlayerMock {
        val player = PlayerMock(server, javaName, uuid)
        server.addPlayer(player)
        return player
    }

    @Test
    fun `collectPlayers returns empty list when no players tracked`() {
        val players = provider.collectPlayers()
        assertTrue(players.isEmpty())
    }

    @Test
    fun `collectPlayers returns correct player data after addPlayer`() {
        val world = server.addSimpleWorld("world")
        val player = server.addPlayer("TestPlayer")

        player.teleport(world.spawnLocation.add(100.0, 64.0, 200.0))

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

        player1.teleport(world.spawnLocation.add(100.0, 64.0, 200.0))
        player2.teleport(world.spawnLocation.add(50.0, 70.0, 150.0))

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

        player.disconnect()

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
     * The BVC client always signs in with the Xbox gamertag, so the server indexes a
     * linked player under it alone. Their Java name is not a second key.
     */
    @Test
    fun `a linked player is keyed by their gamertag`() {
        val player = server.addPlayer("JavaName")
        val linked = providerWith(mapOf(player.uniqueId to "XboxTag"))

        assertEquals("XboxTag", linked.resolveCanonicalName(player))
        assertEquals(listOf("minecraft:XboxTag"), linked.resolveMembershipKeys(player))
    }

    /**
     * A Floodgate-prefixed player is already known by their gamertag alone, so the
     * prefixed Java username is not a second identity — it is the same one wearing a
     * prefix, and nothing registers under it.
     */
    @Test
    fun `a prefixed bedrock player offers only their gamertag`() {
        val provider = providerWith(mapOf(floodgate to "XboxTag"))
        val player = floodgatePlayer(".XboxTag")

        assertEquals(listOf("minecraft:XboxTag"), provider.resolveMembershipKeys(player))
    }

    // Floodgate's default replace-spaces turns "Player One" into ".Player_One".
    // That is still the prefixed form of the gamertag, not a linked Java account. The
    // quit phantom (PaperPlugin.onPlayerQuit) takes this same name.
    @Test
    fun `a prefixed bedrock player whose gamertag has a space is known by their gamertag`() {
        val provider = providerWith(mapOf(floodgate to "Player One"))
        val player = floodgatePlayer(".Player_One")

        assertEquals("Player One", provider.resolveCanonicalName(player))
        assertEquals(listOf("minecraft:Player One"), provider.resolveMembershipKeys(player))
    }

    // Floodgate cuts prefix + gamertag to 16 characters.
    @Test
    fun `a prefixed bedrock player whose name was truncated is known by their gamertag`() {
        val provider = providerWith(mapOf(floodgate to "LongGamertag123"))
        val player = floodgatePlayer("BE_LongGamertag1")

        assertEquals("LongGamertag123", provider.resolveCanonicalName(player))
    }

    @Test
    fun `an unlinked floodgate player is reported under their gamertag`() {
        val provider = providerWith(mapOf(floodgate to "Player One"))
        val player = floodgatePlayer(".Player_One")
        provider.addPlayer(player)

        val reported = provider.collectPlayers().single()

        assertEquals("Player One", reported.name)
        assertNull(reported.alternativeIdentity)
    }

    // The shape beta.27 sent, so an older server behaves as it did and a newer one keeps
    // the Java name as an alias.
    @Test
    fun `a linked player is reported in the beta27 shape`() {
        val player = server.addPlayer("JavaName")
        val provider = providerWith(mapOf(player.uniqueId to "XboxTag"))
        provider.addPlayer(player)

        val reported = provider.collectPlayers().single()

        assertEquals("JavaName", reported.name)
        assertEquals("XboxTag", reported.alternativeIdentity)
    }

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
        val provider = providerWith(mapOf(floodgate to "XboxTag"))
        val player = floodgatePlayer(".XboxTag")

        assertEquals(player.uniqueId, provider.findByIdentity("XboxTag")?.uniqueId)
    }

    // A linked player is named by their Java account here and may be named by their
    // gamertag on the far side, so both have to come back to the same body.
    @Test
    fun `a linked player is found by either of their names`() {
        val player = server.addPlayer("JavaName")
        val provider = providerWith(mapOf(player.uniqueId to "XboxTag"))

        assertEquals(player.uniqueId, provider.findByIdentity("JavaName")?.uniqueId)
        assertEquals(player.uniqueId, provider.findByIdentity("XboxTag")?.uniqueId)
    }

    // A linked player's Java name may be another online player's gamertag. The owner of
    // the gamertag must be found, or SVC places their audio at the linked player's body.
    @Test
    fun `a gamertag finds its owner before a linked players java name`() {
        val linked = server.addPlayer("Notch")
        val bedrock = floodgatePlayer(".Notch", floodgate2)
        val provider = providerWith(
            mapOf(linked.uniqueId to "LongGamertag123", floodgate2 to "Notch")
        )

        assertEquals(bedrock.uniqueId, provider.findByIdentity("Notch")?.uniqueId)
        assertEquals(linked.uniqueId, provider.findByIdentity("LongGamertag123")?.uniqueId)
    }

    // A speaker on another server has no body here, and inventing one would attach
    // their audio to a stranger.
    @Test
    fun `a name nobody answers to finds no body`() {
        server.addPlayer("TestPlayer")

        assertNull(provider.findByIdentity("SomeoneElse"))
    }

    /**
     * One name in, one key out. A player whose two names agree must not be offered
     * twice, or every lookup for them costs two.
     */
    @Test
    fun `a player whose names agree is offered once`() {
        val player = server.addPlayer("SameName")
        val provider = providerWith(mapOf(player.uniqueId to "SameName"))

        assertEquals(listOf("minecraft:SameName"), provider.resolveMembershipKeys(player))
    }
}
