package com.alaydriem.bedrockvoicechat.fabric;

import com.alaydriem.bedrockvoicechat.api.ConfigProvider;
import com.alaydriem.bedrockvoicechat.config.ModConfig;
import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import net.fabricmc.loader.api.FabricLoader;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import java.io.Reader;
import java.io.Writer;
import java.nio.file.Files;
import java.nio.file.Path;

/**
 * Fabric-specific configuration provider using JSON files.
 */
public class FabricConfigProvider implements ConfigProvider {
    private static final Logger LOGGER = LoggerFactory.getLogger("Bedrock Voice Chat");
    private static final Path CONFIG_PATH =
            FabricLoader.getInstance().getConfigDir().resolve("bedrock-voice-chat.json");
    private static final Gson GSON = new GsonBuilder().setPrettyPrinting().create();

    @Override
    public ModConfig load() {
        if (!Files.exists(CONFIG_PATH)) {
            LOGGER.warn("Config not found, creating default config");
            createDefaultIfMissing();
            return new ModConfig();
        }

        try (Reader reader = Files.newBufferedReader(CONFIG_PATH)) {
            return GSON.fromJson(reader, ModConfig.class);
        } catch (Exception e) {
            LOGGER.error("Failed to load config", e);
            return new ModConfig();
        }
    }

    @Override
    public void save(ModConfig config) {
        try (Writer writer = Files.newBufferedWriter(CONFIG_PATH)) {
            GSON.toJson(config, writer);
        } catch (Exception e) {
            LOGGER.error("Failed to save config", e);
        }
    }

    @Override
    public void createDefaultIfMissing() {
        if (Files.exists(CONFIG_PATH)) {
            return;
        }

        ModConfig defaultConfig = new ModConfig();
        defaultConfig.setBvcServer("");
        defaultConfig.setAccessToken("");
        defaultConfig.setMinimumPlayers(2);

        try (Writer writer = Files.newBufferedWriter(CONFIG_PATH)) {
            GSON.toJson(defaultConfig, writer);
            LOGGER.info("Default config created at: {}", CONFIG_PATH);
        } catch (Exception e) {
            LOGGER.error("Failed to create default config", e);
        }
    }
}
