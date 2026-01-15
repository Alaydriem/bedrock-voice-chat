package com.alaydriem.bedrockvoicechat.hytale

import com.hypixel.hytale.codec.Codec
import com.hypixel.hytale.codec.KeyedCodec
import com.hypixel.hytale.codec.builder.BuilderCodec

/**
 * Hytale-native config class using BuilderCodec.
 * Mirrors ModConfig fields but uses Hytale's serialization.
 */
class HytaleModConfig {
    var bvcServer: String = ""
    var accessToken: String = ""
    var minimumPlayers: Int = 2

    fun isValid(): Boolean =
        bvcServer.isNotBlank() && accessToken.isNotBlank()

    companion object {
        /**
         * Lazily initialized codec to avoid ClassNotFoundException during testing.
         * Only initialized when actually needed (at plugin runtime).
         */
        @JvmStatic
        val CODEC: BuilderCodec<HytaleModConfig> by lazy {
            BuilderCodec.builder(
                HytaleModConfig::class.java,
                ::HytaleModConfig
            )
                .append(
                    KeyedCodec("bvc-server", Codec.STRING),
                    { cfg, value -> cfg.bvcServer = value },
                    { cfg -> cfg.bvcServer }
                ).add()
                .append(
                    KeyedCodec("access-token", Codec.STRING),
                    { cfg, value -> cfg.accessToken = value },
                    { cfg -> cfg.accessToken }
                ).add()
                .append(
                    KeyedCodec("minimum-players", Codec.INTEGER),
                    { cfg, value -> cfg.minimumPlayers = value },
                    { cfg -> cfg.minimumPlayers }
                ).add()
                .build()
        }
    }
}
