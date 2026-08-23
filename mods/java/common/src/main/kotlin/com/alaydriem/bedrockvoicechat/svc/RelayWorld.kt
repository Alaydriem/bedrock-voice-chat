package com.alaydriem.bedrockvoicechat.svc

import org.slf4j.LoggerFactory
import java.io.File
import java.util.UUID

/**
 * The identifier this Minecraft server declares to its BVC peer.
 *
 * One per server rather than per dimension: a relay world is the set of players who
 * share a proximity space *across* servers, and dimension and world_uuid already
 * separate players within one.
 *
 * Minted and persisted rather than computed. The two platforms already derive
 * world_uuid by different mechanisms — Bukkit's per-world uid on Paper, a persisted
 * UUID per dimension on Fabric — and a value the mod and the bridge must both
 * produce should have one origin rather than two derivations that can drift.
 *
 * The operator never sees it: `worlds` is optional in config.hcl precisely so nobody
 * has to copy an identifier only the far side can see.
 */
class RelayWorld(private val dataDir: File) {

    @Volatile
    private var cached: String? = null

    @Synchronized
    fun id(): String {
        cached?.let { return it }

        dataDir.mkdirs()
        val file = File(dataDir, FILE_NAME)

        // A blank file is treated as absent. A truncated write leaves one, and
        // returning "" from it would declare an empty world at handshake — which
        // the far side cannot grant, and which reads as a misconfiguration rather
        // than as the corrupt file it is.
        val value = if (file.isFile) {
            file.readText().trim().ifEmpty { mint(file) }
        } else {
            mint(file)
        }

        cached = value
        return value
    }

    private fun mint(file: File): String {
        val value = UUID.randomUUID().toString()
        file.writeText(value)
        logger.info("Minted this server's relay world identifier")
        return value
    }

    companion object {
        private const val FILE_NAME: String = "relay_world.txt"

        private val logger = LoggerFactory.getLogger("BVC Relay")
    }
}
