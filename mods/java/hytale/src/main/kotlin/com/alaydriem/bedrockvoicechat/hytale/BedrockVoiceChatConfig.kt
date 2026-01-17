package com.alaydriem.bedrockvoicechat.hytale

import com.hypixel.hytale.codec.Codec
import com.hypixel.hytale.codec.KeyedCodec
import com.hypixel.hytale.codec.builder.BuilderCodec

/**
 * Hytale-native config class using BuilderCodec.
 */
class BedrockVoiceChatConfig {
    @JvmField var bvcServer: String = ""
    @JvmField var accessToken: String = ""
    @JvmField var minimumPlayers: Int = 2

    fun isValid(): Boolean =
        bvcServer.isNotBlank() && accessToken.isNotBlank()

    companion object {
        @JvmStatic
        val CODEC: BuilderCodec<BedrockVoiceChatConfig> = BuilderCodec.builder(
            BedrockVoiceChatConfig::class.java
        ) { BedrockVoiceChatConfig() }
            .append(
                KeyedCodec<String>("BvcServer", Codec.STRING),
                { cfg, value, _ -> cfg.bvcServer = value },
                { cfg, _ -> cfg.bvcServer }
            ).add()
            .append(
                KeyedCodec<String>("AccessToken", Codec.STRING),
                { cfg, value, _ -> cfg.accessToken = value },
                { cfg, _ -> cfg.accessToken }
            ).add()
            .append(
                KeyedCodec<Int>("MinimumPlayers", Codec.INTEGER),
                { cfg, value, _ -> cfg.minimumPlayers = value },
                { cfg, _ -> cfg.minimumPlayers }
            ).add()
            .build()
    }
}
