package com.alaydriem.bedrockvoicechat.paper;

import com.alaydriem.bedrockvoicechat.api.ConfigProvider;
import com.alaydriem.bedrockvoicechat.config.ModConfig;
import org.bukkit.configuration.file.FileConfiguration;
import org.bukkit.plugin.java.JavaPlugin;

/**
 * Paper-specific configuration provider using YAML files.
 */
public class PaperConfigProvider implements ConfigProvider {
    private final JavaPlugin plugin;

    public PaperConfigProvider(JavaPlugin plugin) {
        this.plugin = plugin;
    }

    @Override
    public ModConfig load() {
        FileConfiguration yamlConfig = plugin.getConfig();

        ModConfig config = new ModConfig();
        config.setBvcServer(yamlConfig.getString("bvc-server", ""));
        config.setAccessToken(yamlConfig.getString("access-token", ""));
        config.setMinimumPlayers(yamlConfig.getInt("minimum-players", 2));

        return config;
    }

    @Override
    public void save(ModConfig config) {
        FileConfiguration yamlConfig = plugin.getConfig();
        yamlConfig.set("bvc-server", config.getBvcServer());
        yamlConfig.set("access-token", config.getAccessToken());
        yamlConfig.set("minimum-players", config.getMinimumPlayers());
        plugin.saveConfig();
    }

    @Override
    public void createDefaultIfMissing() {
        plugin.saveDefaultConfig();
    }
}
