package com.alaydriem.bedrockvoicechat.powernukkitx

import com.alaydriem.bedrockvoicechat.api.ConfigProvider
import com.alaydriem.bedrockvoicechat.config.LegacyEmbeddedKeys
import com.alaydriem.bedrockvoicechat.config.ModConfig
import com.google.gson.Gson
import org.powernukkitx.plugin.PluginBase
import org.powernukkitx.utils.Config
import java.nio.file.Path

/**
 * PowerNukkitX configuration provider, reading `config.yml` in the plugin data
 * folder with the same keys as the Paper plugin.
 *
 * The YAML document is converted whole and deserialized in one step, so every block
 * the mod understands is read without this class knowing any key by name.
 */
class PnxConfigProvider(private val plugin: PluginBase) : ConfigProvider {

    override fun getConfigDir(): Path = plugin.getDataFolder().toPath()

    override fun load(): ModConfig = fromConfig(plugin.getConfig())

    override fun createDefaultIfMissing() {
        plugin.saveDefaultConfig()
    }

    companion object {
        private val GSON = Gson()

        @JvmStatic
        fun fromConfig(source: Config): ModConfig {
            val json = GSON.toJsonTree(source.getAll()).asJsonObject
            val config = GSON.fromJson(json, ModConfig::class.java) ?: ModConfig()
            val embedded = json.getAsJsonObject("embedded-config")
                ?: json.getAsJsonObject("embeddedConfig")
            config.legacyKeys = LegacyEmbeddedKeys.detect(embedded)
            return config
        }
    }
}
