package com.alaydriem.bedrockvoicechat.powernukkitx

import com.google.gson.JsonParser
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertNotEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.io.TempDir
import java.io.File
import java.io.IOException
import java.util.UUID

class PnxWorldIdentityTest {

    // A level whose identifier changes on restart splits every player in it from
    // the world membership they held before the restart.
    @Test
    fun the_same_level_keeps_its_uuid_across_instances(@TempDir dataFolder: File) {
        val first = PnxWorldIdentity(dataFolder).uuidFor("world")

        val restarted = PnxWorldIdentity(dataFolder)

        assertEquals(first, restarted.uuidFor("world"))
        assertNotEquals(first, restarted.uuidFor("nether"))
    }

    // A directory where the staging file belongs makes the first save fail.
    @Test
    fun a_failed_save_is_retried_after_the_backoff_and_never_changes_the_uuid(@TempDir dataFolder: File) {
        val blocker = File(dataFolder, "worlds.json.tmp")
        File(blocker, "keep").apply { parentFile.mkdirs() }.writeText("x")
        var now = 1_000L
        val identity = PnxWorldIdentity(dataFolder, clock = { now })

        val first = identity.uuidFor("world")
        blocker.deleteRecursively()

        now += 29_999L
        assertEquals(first, identity.uuidFor("world"))
        assertFalse(File(dataFolder, "worlds.json").exists())

        now += 1L
        assertEquals(first, identity.uuidFor("world"))
        assertEquals(first, PnxWorldIdentity(dataFolder).uuidFor("world"))
    }

    @Test
    fun a_corrupt_store_is_backed_up_and_replaced(@TempDir dataFolder: File) {
        val store = File(dataFolder, "worlds.json")
        store.writeText("{")

        val id = PnxWorldIdentity(dataFolder).uuidFor("world")

        UUID.fromString(id)
        val rewritten = JsonParser.parseString(store.readText()).asJsonObject
        assertEquals(id, rewritten.get("world").asString)
        assertEquals("{", File(dataFolder, "worlds.json.bad").readText())
    }

    @Test
    fun a_read_that_fails_briefly_still_returns_the_stored_uuid(@TempDir dataFolder: File) {
        val stored = PnxWorldIdentity(dataFolder).uuidFor("world")
        var failures = 2
        val reader: (File) -> String = {
            if (failures-- > 0) throw IOException("locked") else it.readText()
        }

        assertEquals(stored, PnxWorldIdentity(dataFolder, readStore = reader).uuidFor("world"))
    }

    // Writing after a failed read would replace every stored UUID with new ones.
    @Test
    fun an_unreadable_store_is_never_overwritten(@TempDir dataFolder: File) {
        val stored = PnxWorldIdentity(dataFolder).uuidFor("world")
        val store = File(dataFolder, "worlds.json")
        val before = store.readText()
        val identity = PnxWorldIdentity(dataFolder, readStore = { throw IOException("locked") })

        val temporary = identity.uuidFor("world")
        identity.uuidFor("nether")

        assertNotEquals(stored, temporary)
        assertEquals(temporary, identity.uuidFor("world"))
        assertEquals(before, store.readText())
        assertTrue(PnxWorldIdentity(dataFolder).uuidFor("world") == stored)
    }
}
