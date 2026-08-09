package com.alaydriem.bedrockvoicechat.server

import com.alaydriem.bedrockvoicechat.config.generated.Audio
import com.alaydriem.bedrockvoicechat.config.generated.Database
import com.alaydriem.bedrockvoicechat.config.generated.EmbeddedServerConfig
import com.alaydriem.bedrockvoicechat.config.generated.Minecraft
import com.alaydriem.bedrockvoicechat.config.generated.Server
import com.alaydriem.bedrockvoicechat.config.generated.Tls
import com.google.gson.Gson
import java.util.UUID

/**
 * Fills the values that depend on where the mod put its data directory, and
 * nothing else. Every other key reaches the server exactly as the operator
 * wrote it, or stays absent so the server's own default applies.
 */
class RuntimeConfigBuilder(private val configDir: String) {
    companion object {
        private val GSON = Gson()
        private val SQLITE_SCHEMES = setOf("sqlite", "sqlite3")
    }

    /**
     * The token the embedded server was configured with. The mod authenticates
     * back to the server it started, so a generated token has to be retained.
     */
    var resolvedAccessToken: String = ""
        private set

    fun build(source: EmbeddedServerConfig?, accessToken: String?): EmbeddedServerConfig {
        val config = copyOf(source)

        val server = config.server ?: Server().also { config.server = it }
        val tls = server.tls ?: Tls().also { server.tls = it }
        if (tls.certsPath == null) {
            tls.certsPath = "$configDir/certificates"
        }

        if (server.assetsPath == null) {
            server.assetsPath = "$configDir/assets"
        }

        val audio = config.audio ?: Audio().also { config.audio = it }
        if (audio.filePath == null) {
            audio.filePath = "${server.assetsPath}/audio"
        }

        val database = config.database ?: Database().also { config.database = it }
        val scheme = database.scheme ?: "sqlite3"
        if (database.database == null && SQLITE_SCHEMES.contains(scheme)) {
            database.database = "$configDir/bvc.sqlite3"
        }

        val token = accessToken?.takeIf { it.isNotBlank() } ?: UUID.randomUUID().toString()
        resolvedAccessToken = token
        val minecraft = server.minecraft ?: Minecraft().also { server.minecraft = it }
        minecraft.accessToken = token

        return config
    }

    fun toJson(config: EmbeddedServerConfig): String = GSON.toJson(config)

    // A round trip, so the config the provider loaded is never mutated by the
    // act of starting a server.
    private fun copyOf(source: EmbeddedServerConfig?): EmbeddedServerConfig {
        if (source == null) {
            return EmbeddedServerConfig()
        }
        return GSON.fromJson(GSON.toJson(source), EmbeddedServerConfig::class.java)
            ?: EmbeddedServerConfig()
    }
}
