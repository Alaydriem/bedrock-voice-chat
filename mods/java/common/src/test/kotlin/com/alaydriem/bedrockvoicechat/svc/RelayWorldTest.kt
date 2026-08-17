package com.alaydriem.bedrockvoicechat.svc

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.io.TempDir
import java.io.File

class RelayWorldTest {

    // Declared at handshake, so a value that changed per boot would silently
    // repartition every peered world on every restart.
    @Test
    fun `the identifier survives a restart`(@TempDir dir: File) {
        val first = RelayWorld(dir).id()
        val second = RelayWorld(dir).id()

        assertEquals(first, second)
    }

    // One per server, not per dimension. Dimension and world_uuid already separate
    // players within a server; the relay world decides which peer link carries them.
    @Test
    fun `two servers mint different identifiers`(@TempDir a: File, @TempDir b: File) {
        assertTrue(RelayWorld(a).id() != RelayWorld(b).id())
    }

    @Test
    fun `the identifier is written where a later start will find it`(@TempDir dir: File) {
        val id = RelayWorld(dir).id()

        assertEquals(id, File(dir, "relay_world.txt").readText().trim())
    }

    // A truncated write leaves an empty file. Returning "" from it would declare an
    // empty world at handshake, which the far side cannot grant and which reads as
    // a configuration mistake rather than a corrupt file.
    @Test
    fun `an empty file is replaced rather than returned`(@TempDir dir: File) {
        File(dir, "relay_world.txt").writeText("   ")

        val id = RelayWorld(dir).id()

        assertTrue(id.isNotBlank())
        assertEquals(id, File(dir, "relay_world.txt").readText().trim())
    }
}
