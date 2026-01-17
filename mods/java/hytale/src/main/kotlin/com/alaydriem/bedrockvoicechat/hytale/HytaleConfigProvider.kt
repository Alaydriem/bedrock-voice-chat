package com.alaydriem.bedrockvoicechat.hytale

import com.alaydriem.bedrockvoicechat.api.ConfigProvider
import com.alaydriem.bedrockvoicechat.config.ModConfig
import com.hypixel.hytale.server.core.util.Config

/**
 * Hytale-specific ConfigProvider backed by BuilderCodec.
 * Converts between BedrockVoiceChatConfig and the common ModConfig interface.
 */
class HytaleConfigProvider(private val config: Config<BedrockVoiceChatConfig>) : ConfigProvider {

    override fun load(): ModConfig {
        val hytaleConfig = config.get()
        println("[BVC ConfigProvider] Raw Hytale config: bvcServer='${hytaleConfig.bvcServer}', accessToken='${hytaleConfig.accessToken}', minimumPlayers=${hytaleConfig.minimumPlayers}")

        return ModConfig().apply {
            bvcServer = hytaleConfig.bvcServer
            accessToken = hytaleConfig.accessToken
            minimumPlayers = hytaleConfig.minimumPlayers
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
