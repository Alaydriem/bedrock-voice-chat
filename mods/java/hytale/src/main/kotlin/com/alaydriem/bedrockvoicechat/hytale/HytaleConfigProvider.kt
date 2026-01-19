package com.alaydriem.bedrockvoicechat.hytale

import com.alaydriem.bedrockvoicechat.api.ConfigProvider
import com.alaydriem.bedrockvoicechat.config.EmbeddedConfig
import com.alaydriem.bedrockvoicechat.config.ModConfig
import com.google.gson.Gson
import com.google.gson.JsonObject
import com.hypixel.hytale.server.core.util.Config
import java.nio.file.Path
import java.nio.file.Paths

/**
 * Hytale-specific ConfigProvider backed by BuilderCodec.
 * Converts between BedrockVoiceChatConfig and the common ModConfig interface.
 */
class HytaleConfigProvider(private val config: Config<BedrockVoiceChatConfig>) : ConfigProvider {

    private val pluginAddress: String by lazy {
        val manifestStream = javaClass.classLoader.getResourceAsStream("manifest.json")
            ?: throw IllegalStateException("manifest.json not found in resources")
        val manifest = Gson().fromJson(manifestStream.reader(), JsonObject::class.java)
        val group = manifest.get("Group")?.asString ?: throw IllegalStateException("Group not found in manifest")
        val name = manifest.get("Name")?.asString ?: throw IllegalStateException("Name not found in manifest")
        "${group}_${name}"
    }

    override fun getConfigDir(): Path = Paths.get(System.getProperty("user.dir"), "mods", pluginAddress)

    override fun load(): ModConfig {
        val hytaleConfig = config.get()

        return ModConfig().apply {
            bvcServer = hytaleConfig.bvcServer
            accessToken = hytaleConfig.accessToken
            minimumPlayers = hytaleConfig.minimumPlayers
            useEmbeddedServer = hytaleConfig.useEmbeddedServer

            if (useEmbeddedServer) {
                embeddedConfig = hytaleConfig.embeddedConfig?.let { hytaleEmbed ->
                    EmbeddedConfig().apply {
                        httpPort = hytaleEmbed.httpPort
                        quicPort = hytaleEmbed.quicPort
                        publicAddr = hytaleEmbed.publicAddr
                        broadcastRange = hytaleEmbed.broadcastRange
                        tlsCertificate = hytaleEmbed.tlsCertificate
                        tlsKey = hytaleEmbed.tlsKey
                        tlsNames = hytaleEmbed.getTlsNamesList()
                        tlsIps = hytaleEmbed.getTlsIpsList()
                        logLevel = hytaleEmbed.logLevel
                    }
                } ?: EmbeddedConfig()
            }
        }
    }

    override fun save(config: ModConfig) {
        // Hytale's Config handles persistence automatically
        // This is a no-op as we don't sync back from ModConfig
    }

    override fun createDefaultIfMissing() {
        // Hytale's withConfig() handles this automatically
        // Config file is created with defaults on first access
    }
}
