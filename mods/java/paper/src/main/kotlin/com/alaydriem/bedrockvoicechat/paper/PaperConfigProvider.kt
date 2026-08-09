package com.alaydriem.bedrockvoicechat.paper

import com.alaydriem.bedrockvoicechat.api.ConfigProvider
import com.alaydriem.bedrockvoicechat.config.LegacyEmbeddedKeys
import com.alaydriem.bedrockvoicechat.config.ModConfig
import com.google.gson.Gson
import com.google.gson.JsonObject
import org.bukkit.plugin.java.JavaPlugin
import java.nio.file.Path

/**
 * Paper configuration provider. The YAML document is converted whole and
 * deserialized in one step, so every block the mod understands is read without
 * this class knowing any key by name.
 */
class PaperConfigProvider(private val plugin: JavaPlugin) : ConfigProvider {
    companion object {
        private val GSON = Gson()

        @JvmStatic
        fun fromJson(json: JsonObject): ModConfig {
            val config = GSON.fromJson(json, ModConfig::class.java) ?: ModConfig()
            val embedded = json.getAsJsonObject("embedded-config")
                ?: json.getAsJsonObject("embeddedConfig")
            config.legacyKeys = LegacyEmbeddedKeys.detect(embedded)
            return config
        }
    }

    override fun getConfigDir(): Path = plugin.dataFolder.toPath()

    override fun load(): ModConfig = fromJson(YamlSectionConverter.toJson(plugin.config))

    override fun createDefaultIfMissing() {
        plugin.saveDefaultConfig()
    }
}
