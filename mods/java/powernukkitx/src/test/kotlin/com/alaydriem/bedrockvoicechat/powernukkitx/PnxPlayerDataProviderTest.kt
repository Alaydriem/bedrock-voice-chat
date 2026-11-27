package com.alaydriem.bedrockvoicechat.powernukkitx

import com.alaydriem.bedrockvoicechat.dto.Dimension
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.io.TempDir
import org.mockito.kotlin.doReturn
import org.mockito.kotlin.mock
import org.powernukkitx.Player
import org.powernukkitx.level.Level
import java.io.File
import java.util.UUID

class PnxPlayerDataProviderTest {

    @Test
    fun builds_player_data_from_a_mocked_player(@TempDir dataFolder: File) {
        val playerId = UUID.randomUUID()
        val level = mock<Level> {
            on { getDimension() } doReturn Level.DIMENSION_NETHER
            on { getFolderName() } doReturn "nether"
        }
        val player = mock<Player> {
            on { getName() } doReturn "SomeGamertag"
            on { getUniqueId() } doReturn playerId
            on { isOnline() } doReturn true
            on { getLevel() } doReturn level
            on { getX() } doReturn 12.5
            on { getY() } doReturn 64.0
            on { getZ() } doReturn -30.25
            on { getYaw() } doReturn 90.0
            on { getPitch() } doReturn 10.0
            on { isSneaking() } doReturn true
            on { isSpectator() } doReturn false
        }
        val worlds = PnxWorldIdentity(dataFolder)
        val provider = PnxPlayerDataProvider(worlds, null) { id -> player.takeIf { id == playerId } }

        provider.addPlayer(player)
        val players = provider.collectPlayers()

        assertEquals(1, players.size)
        val data = players.single()
        assertEquals("SomeGamertag", data.name)
        assertEquals(12.5, data.coordinates.x)
        assertEquals(64.0, data.coordinates.y)
        assertEquals(-30.25, data.coordinates.z)
        assertEquals(Dimension.Minecraft.NETHER.toApiString(), data.dimension)
        assertEquals(PnxWorldIdentity(dataFolder).uuidFor("nether"), data.worldUuid)
        assertEquals(playerId.toString(), data.playerUuid)
        assertTrue(data.whispering)
        assertFalse(data.spectator)
    }

    // A dead player keeps their dimension and position and is flagged a spectator.
    @Test
    fun a_dead_player_keeps_their_place_as_a_spectator(@TempDir dataFolder: File) {
        val playerId = UUID.randomUUID()
        val level = mock<Level> {
            on { getDimension() } doReturn Level.DIMENSION_OVERWORLD
            on { getFolderName() } doReturn "world"
        }
        val player = mock<Player> {
            on { getName() } doReturn "SomeGamertag"
            on { getUniqueId() } doReturn playerId
            on { isOnline() } doReturn true
            on { isSneaking() } doReturn true
            on { getLevel() } doReturn level
            on { getX() } doReturn 12.5
            on { getY() } doReturn 64.0
            on { getZ() } doReturn -30.25
        }
        val provider = PnxPlayerDataProvider(PnxWorldIdentity(dataFolder), null) { id -> player.takeIf { id == playerId } }

        provider.addPlayer(player)
        provider.markDead(player)
        val data = provider.collectPlayers().single()

        assertEquals(Dimension.Minecraft.OVERWORLD.toApiString(), data.dimension)
        assertEquals(12.5, data.coordinates.x)
        assertEquals(64.0, data.coordinates.y)
        assertEquals(-30.25, data.coordinates.z)
        assertTrue(data.spectator)
        assertFalse(data.whispering)

        provider.markAlive(player)
        assertFalse(provider.collectPlayers().single().spectator)
    }
}
