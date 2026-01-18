package com.alaydriem.bedrockvoicechat.hytale

import com.hypixel.hytale.codec.Codec
import com.hypixel.hytale.codec.KeyedCodec
import com.hypixel.hytale.codec.builder.BuilderCodec

/**
 * Embedded server configuration for Hytale using BuilderCodec.
 * Note: tlsNames and tlsIps are comma-separated strings due to codec limitations.
 */
class HytaleEmbeddedConfig {
    @JvmField var httpPort: Int = 443
    @JvmField var quicPort: Int = 8443
    @JvmField var publicAddr: String = "127.0.0.1"
    @JvmField var broadcastRange: Float = 32.0f
    /** Comma-separated list of TLS DNS names (e.g., "localhost,127.0.0.1") */
    @JvmField var tlsNames: String = "localhost,127.0.0.1"
    /** Comma-separated list of TLS IP addresses (e.g., "127.0.0.1") */
    @JvmField var tlsIps: String = "127.0.0.1"
    @JvmField var logLevel: String = "info"

    /** Parse tlsNames as a list */
    fun getTlsNamesList(): List<String> = tlsNames.split(",").map { it.trim() }.filter { it.isNotEmpty() }

    /** Parse tlsIps as a list */
    fun getTlsIpsList(): List<String> = tlsIps.split(",").map { it.trim() }.filter { it.isNotEmpty() }

    companion object {
        @JvmStatic
        val CODEC: BuilderCodec<HytaleEmbeddedConfig> = BuilderCodec.builder(
            HytaleEmbeddedConfig::class.java
        ) { HytaleEmbeddedConfig() }
            .append(
                KeyedCodec<Int>("HttpPort", Codec.INTEGER),
                { cfg, value, _ -> cfg.httpPort = value },
                { cfg, _ -> cfg.httpPort }
            ).add()
            .append(
                KeyedCodec<Int>("QuicPort", Codec.INTEGER),
                { cfg, value, _ -> cfg.quicPort = value },
                { cfg, _ -> cfg.quicPort }
            ).add()
            .append(
                KeyedCodec<String>("PublicAddr", Codec.STRING),
                { cfg, value, _ -> cfg.publicAddr = value },
                { cfg, _ -> cfg.publicAddr }
            ).add()
            .append(
                KeyedCodec<Float>("BroadcastRange", Codec.FLOAT),
                { cfg, value, _ -> cfg.broadcastRange = value },
                { cfg, _ -> cfg.broadcastRange }
            ).add()
            .append(
                KeyedCodec<String>("TlsNames", Codec.STRING),
                { cfg, value, _ -> cfg.tlsNames = value },
                { cfg, _ -> cfg.tlsNames }
            ).add()
            .append(
                KeyedCodec<String>("TlsIps", Codec.STRING),
                { cfg, value, _ -> cfg.tlsIps = value },
                { cfg, _ -> cfg.tlsIps }
            ).add()
            .append(
                KeyedCodec<String>("LogLevel", Codec.STRING),
                { cfg, value, _ -> cfg.logLevel = value },
                { cfg, _ -> cfg.logLevel }
            ).add()
            .build()
    }
}

/**
 * Hytale-native config class using BuilderCodec.
 */
class BedrockVoiceChatConfig {
    @JvmField var bvcServer: String = ""
    @JvmField var accessToken: String = ""
    @JvmField var minimumPlayers: Int = 2
    @JvmField var useEmbeddedServer: Boolean = false
    @JvmField var embeddedConfig: HytaleEmbeddedConfig? = null

    fun isValid(): Boolean = when {
        useEmbeddedServer -> true
        else -> bvcServer.isNotBlank() && accessToken.isNotBlank()
    }

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
            .append(
                KeyedCodec<Boolean>("UseEmbeddedServer", Codec.BOOLEAN),
                { cfg, value, _ -> cfg.useEmbeddedServer = value },
                { cfg, _ -> cfg.useEmbeddedServer }
            ).add()
            .append(
                KeyedCodec<HytaleEmbeddedConfig>("EmbeddedConfig", HytaleEmbeddedConfig.CODEC),
                { cfg, value, _ -> cfg.embeddedConfig = value },
                { cfg, _ -> cfg.embeddedConfig }
            ).add()
            .build()
    }
}
