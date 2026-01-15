package com.alaydriem.bedrockvoicechat.hytale;

import com.alaydriem.bedrockvoicechat.api.ConfigProvider;
import com.alaydriem.bedrockvoicechat.config.ModConfig;
import com.hypixel.hytale.server.core.util.Config;

/**
 * Hytale-specific ConfigProvider backed by BuilderCodec.
 * Converts between HytaleModConfig and the common ModConfig interface.
 */
public class HytaleConfigProvider implements ConfigProvider {
    private final Config<HytaleModConfig> config;

    public HytaleConfigProvider(Config<HytaleModConfig> config) {
        this.config = config;
    }

    @Override
    public ModConfig load() {
        HytaleModConfig hytaleConfig = config.get();

        // Convert to common ModConfig
        ModConfig modConfig = new ModConfig();
        modConfig.setBvcServer(hytaleConfig.getBvcServer());
        modConfig.setAccessToken(hytaleConfig.getAccessToken());
        modConfig.setMinimumPlayers(hytaleConfig.getMinimumPlayers());

        return modConfig;
    }

    @Override
    public void save(ModConfig config) {
        // Hytale's Config handles persistence automatically
        // This is a no-op as we don't sync back from ModConfig
    }

    @Override
    public void createDefaultIfMissing() {
        // Hytale's withConfig() handles this automatically
        // Config file is created with defaults on first access
    }
}
