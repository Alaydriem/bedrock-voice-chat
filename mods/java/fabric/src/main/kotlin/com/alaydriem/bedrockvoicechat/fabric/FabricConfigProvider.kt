package com.alaydriem.bedrockvoicechat.fabric

import com.alaydriem.bedrockvoicechat.api.ConfigProvider
import com.alaydriem.bedrockvoicechat.config.ModConfig
import com.google.gson.GsonBuilder
import net.fabricmc.loader.api.FabricLoader
import org.slf4j.LoggerFactory
import java.nio.file.Files

/**
 * Fabric-specific configuration provider using JSON files.
 */
class FabricConfigProvider : ConfigProvider {
    companion object {
        private val LOGGER = LoggerFactory.getLogger("Bedrock Voice Chat")
        private val CONFIG_PATH = FabricLoader.getInstance().configDir.resolve("bedrock-voice-chat.json")
        private val GSON = GsonBuilder().setPrettyPrinting().create()
    }

    override fun load(): ModConfig {
        if (!Files.exists(CONFIG_PATH)) {
            LOGGER.warn("Config not found, creating default config")
            createDefaultIfMissing()
            return ModConfig()
        }

        return try {
            Files.newBufferedReader(CONFIG_PATH).use { reader ->
                GSON.fromJson(reader, ModConfig::class.java)
            }
        } catch (e: Exception) {
            LOGGER.error("Failed to load config", e)
            ModConfig()
        }
    }

    override fun save(config: ModConfig) {
        try {
            Files.newBufferedWriter(CONFIG_PATH).use { writer ->
                GSON.toJson(config, writer)
            }
        } catch (e: Exception) {
            LOGGER.error("Failed to save config", e)
        }
    }

    override fun createDefaultIfMissing() {
        if (Files.exists(CONFIG_PATH)) {
            return
        }

        val defaultConfig = ModConfig().apply {
            bvcServer = ""
            accessToken = ""
            minimumPlayers = 2
        }

        try {
            Files.newBufferedWriter(CONFIG_PATH).use { writer ->
                GSON.toJson(defaultConfig, writer)
            }
            LOGGER.info("Default config created at: {}", CONFIG_PATH)
        } catch (e: Exception) {
            LOGGER.error("Failed to create default config", e)
        }
    }
}
