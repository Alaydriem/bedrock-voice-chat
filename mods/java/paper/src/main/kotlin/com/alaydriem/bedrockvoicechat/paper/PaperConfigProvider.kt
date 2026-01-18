package com.alaydriem.bedrockvoicechat.paper

import com.alaydriem.bedrockvoicechat.api.ConfigProvider
import com.alaydriem.bedrockvoicechat.config.EmbeddedConfig
import com.alaydriem.bedrockvoicechat.config.ModConfig
import org.bukkit.plugin.java.JavaPlugin
import java.nio.file.Path

/**
 * Paper-specific configuration provider using YAML files.
 */
class PaperConfigProvider(private val plugin: JavaPlugin) : ConfigProvider {

    override fun getConfigDir(): Path = plugin.dataFolder.toPath()

    override fun load(): ModConfig {
        val yamlConfig = plugin.config

        return ModConfig().apply {
            // Support both hyphenated and camelCase keys
            bvcServer = yamlConfig.getString("bvc-server")
                ?: yamlConfig.getString("bvcServer", "")
            accessToken = yamlConfig.getString("access-token")
                ?: yamlConfig.getString("accessToken", "")
            minimumPlayers = if (yamlConfig.contains("minimum-players"))
                yamlConfig.getInt("minimum-players", 2)
            else
                yamlConfig.getInt("minimumPlayers", 2)

            // Embedded server settings
            useEmbeddedServer = yamlConfig.getBoolean("use-embedded-server", false)
                || yamlConfig.getBoolean("useEmbeddedServer", false)

            if (useEmbeddedServer) {
                embeddedConfig = EmbeddedConfig().apply {
                    httpPort = yamlConfig.getInt("embedded.http-port",
                        yamlConfig.getInt("embedded.httpPort", 443))
                    quicPort = yamlConfig.getInt("embedded.quic-port",
                        yamlConfig.getInt("embedded.quicPort", 8443))
                    publicAddr = yamlConfig.getString("embedded.public-addr")
                        ?: yamlConfig.getString("embedded.publicAddr", "127.0.0.1")!!
                    broadcastRange = yamlConfig.getDouble("embedded.broadcast-range",
                        yamlConfig.getDouble("embedded.broadcastRange", 32.0)).toFloat()
                    tlsNames = yamlConfig.getStringList("embedded.tls-names").ifEmpty {
                        yamlConfig.getStringList("embedded.tlsNames").ifEmpty {
                            listOf("localhost", "127.0.0.1")
                        }
                    }
                    tlsIps = yamlConfig.getStringList("embedded.tls-ips").ifEmpty {
                        yamlConfig.getStringList("embedded.tlsIps").ifEmpty {
                            listOf("127.0.0.1")
                        }
                    }
                    logLevel = yamlConfig.getString("embedded.log-level")
                        ?: yamlConfig.getString("embedded.logLevel", "info")!!
                }
            }
        }
    }

    override fun save(config: ModConfig) {
        val yamlConfig = plugin.config
        yamlConfig.set("bvc-server", config.bvcServer)
        yamlConfig.set("access-token", config.accessToken)
        yamlConfig.set("minimum-players", config.minimumPlayers)
        yamlConfig.set("use-embedded-server", config.useEmbeddedServer)

        config.embeddedConfig?.let { embedded ->
            yamlConfig.set("embedded.http-port", embedded.httpPort)
            yamlConfig.set("embedded.quic-port", embedded.quicPort)
            yamlConfig.set("embedded.public-addr", embedded.publicAddr)
            yamlConfig.set("embedded.broadcast-range", embedded.broadcastRange)
            yamlConfig.set("embedded.tls-names", embedded.tlsNames)
            yamlConfig.set("embedded.tls-ips", embedded.tlsIps)
            yamlConfig.set("embedded.log-level", embedded.logLevel)
        }

        plugin.saveConfig()
    }

    override fun createDefaultIfMissing() {
        plugin.saveDefaultConfig()
    }
}
