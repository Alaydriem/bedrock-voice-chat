package com.alaydriem.bedrockvoicechat.integration

import com.alaydriem.bedrockvoicechat.dto.Dimension
import com.alaydriem.bedrockvoicechat.dto.PlayerData
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Test
import org.junit.jupiter.params.ParameterizedTest
import org.junit.jupiter.params.provider.MethodSource
import java.util.UUID

class PlayerIdentityResolverTest {
    companion object {
        // Floodgate stores an unlinked Bedrock player under UUID(0, xuid).
        val FLOODGATE: UUID = UUID(0L, 2535428000000001L)
        // A linked player is known by their Java account's UUID.
        val JAVA: UUID = UUID.fromString("3c9a1b77-2d44-4e18-8b6c-000000000002")

        data class GateRow(
            val id: String,
            val gamertag: String,
            val linkedJavaName: String?,
            val prefix: String,
            val replaceSpaces: Boolean,
            val expectedWire: Pair<String, String?>,
            val expectedLocal: String
        ) {
            override fun toString() = id
        }

        @JvmStatic
        fun gateRows(): List<GateRow> = listOf(
            GateRow("G01", "Steve", null, ".", true, "Steve" to null, "Steve"),
            GateRow("G02", "Player One", null, ".", true, "Player One" to null, "Player One"),
            GateRow("G03", "A B C D", null, ".", true, "A B C D" to null, "A B C D"),
            GateRow("G04", "Lord Of Pixels1", null, ".", true, "Lord Of Pixels1" to null, "Lord Of Pixels1"),
            GateRow("G05", "LongGamertag123", null, ".", true, "LongGamertag123" to null, "LongGamertag123"),
            GateRow("G06", "LongGamertag123", null, "BE_", true, "LongGamertag123" to null, "LongGamertag123"),
            GateRow("G07", "Steve", null, "BE_", true, "Steve" to null, "Steve"),
            GateRow("G08", "Player One", null, ".", false, "Player One" to null, "Player One"),
            GateRow("G09", "Steve", null, "", true, "Steve" to null, "Steve"),
            GateRow("G10", "Player One", null, "", true, "Player One" to null, "Player One"),
            GateRow("G11", "テスト ユーザー", null, ".", true, "テスト ユーザー" to null, "テスト ユーザー"),
            GateRow("G12", "Player One", "Linked_Java", "", false, "Linked_Java" to "Player One", "Player One"),
            GateRow("G13", "Steve", "Steve", "", false, "Steve" to null, "Steve"),
            GateRow("G14", "LongGamertag123", "Notch", "", false, "Notch" to "LongGamertag123", "LongGamertag123"),
            GateRow("G17", "Notch", "TheNotch", "", false, "Notch" to null, "Notch")
        )
    }

    @ParameterizedTest(name = "{0}")
    @MethodSource("gateRows")
    fun gate(row: GateRow) {
        val resolver = PlayerIdentityResolver { row.gamertag }
        val identity = if (row.linkedJavaName != null) {
            resolver.resolve(JAVA, row.linkedJavaName)
        } else {
            resolver.resolve(FLOODGATE, FloodgateJavaName.of(row.gamertag, row.prefix, row.replaceSpaces))
        }

        assertEquals(row.expectedLocal, identity.name, "${row.id} local name")
        assertEquals(row.expectedWire, identity.reportedName to identity.reportedAlternative, "${row.id} wire")
    }

    @Test
    fun `the floodgate name fixture matches floodgate`() {
        assertEquals(".Player_One", FloodgateJavaName.of("Player One", ".", true))
        assertEquals("BE_LongGamertag1", FloodgateJavaName.of("LongGamertag123", "BE_", true))
    }

    @Test
    fun `a player floodgate does not know keeps their java name`() {
        val identity = PlayerIdentityResolver { null }.resolve(JAVA, "Steve")

        assertEquals(PlayerIdentity("Steve", null, false), identity)
        assertEquals("Steve" to null, identity.reportedName to identity.reportedAlternative)
    }

    @Test
    fun `a linked bedrock player is named by their gamertag and reported by their java name`() {
        val identity = PlayerIdentityResolver { "XboxTag" }.resolve(JAVA, "JavaName")

        assertEquals("XboxTag", identity.name)
        assertEquals("JavaName" to "XboxTag", identity.reportedName to identity.reportedAlternative)
        assertEquals(listOf("XboxTag", "JavaName"), identity.names)
    }

    // beta.27 read this player as "prefixed" and reported (Notch, -). Keeping that output
    // means an old server still registers them under the name their client signs in with.
    @Test
    fun `a linked player whose java name ends with the gamertag keeps the beta27 wire`() {
        val identity = PlayerIdentityResolver { "Notch" }.resolve(JAVA, "TheNotch")

        assertEquals("Notch", identity.name)
        assertEquals("TheNotch", identity.javaName)
        assertEquals("Notch" to null, identity.reportedName to identity.reportedAlternative)
    }

    // Floodgate on the proxy only: the backend has no API to ask, so nothing better than the
    // Java name is available.
    @Test
    fun `a floodgate player on a server without the floodgate api keeps their java name`() {
        val identity = PlayerIdentityResolver { null }.resolve(FLOODGATE, ".Player_One")

        assertEquals(PlayerIdentity(".Player_One", null, false), identity)
    }

    // A linked player's Java name may be someone else's gamertag. The owner of the gamertag
    // wins, or SVC places one player's audio at another player's body.
    @Test
    fun `bestMatch prefers a gamertag over a java name`() {
        val resolver = PlayerIdentityResolver { null }
        val candidates = listOf(
            1 to PlayerIdentity("LongGamertag123", "Notch", true),
            2 to PlayerIdentity("Notch", ".Notch", false)
        )

        assertEquals(2, resolver.bestMatch("Notch", candidates))
        assertEquals(2, resolver.bestMatch("notch", candidates))
        assertEquals(1, resolver.bestMatch("LongGamertag123", candidates))
        assertNull(resolver.bestMatch("Nobody", candidates))
    }

    @Test
    fun `the playerdata factory projects the wire for linked and unlinked players`() {
        val linked = PlayerData(
            PlayerIdentity("Player One", "Linked_Java", true),
            0.0, 64.0, 0.0, 0f, 0f, Dimension.Minecraft.OVERWORLD, false
        )
        val unlinked = PlayerData(
            PlayerIdentity("Steve", ".Steve", false),
            0.0, 64.0, 0.0, 0f, 0f, Dimension.Minecraft.OVERWORLD, false
        )

        assertEquals("Linked_Java" to "Player One", linked.name to linked.alternativeIdentity)
        assertEquals("Steve" to null, unlinked.name to unlinked.alternativeIdentity)
    }
}
