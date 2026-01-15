package com.alaydriem.bedrockvoicechat.paper

import com.alaydriem.bedrockvoicechat.api.ConfigProvider
import com.alaydriem.bedrockvoicechat.config.ModConfig
import org.bukkit.plugin.java.JavaPlugin

/**
 * Paper-specific configuration provider using YAML files.
 */
class PaperConfigProvider(private val plugin: JavaPlugin) : ConfigProvider {

    override fun load(): ModConfig {
        val yamlConfig = plugin.config

        return ModConfig().apply {
            bvcServer = yamlConfig.getString("bvc-server", "")
            accessToken = yamlConfig.getString("access-token", "")
            minimumPlayers = yamlConfig.getInt("minimum-players", 2)
        }
    }

    override fun save(config: ModConfig) {
        val yamlConfig = plugin.config
        yamlConfig.set("bvc-server", config.bvcServer)
        yamlConfig.set("access-token", config.accessToken)
        yamlConfig.set("minimum-players", config.minimumPlayers)
        plugin.saveConfig()
    }

    override fun createDefaultIfMissing() {
        plugin.saveDefaultConfig()
    }
}
