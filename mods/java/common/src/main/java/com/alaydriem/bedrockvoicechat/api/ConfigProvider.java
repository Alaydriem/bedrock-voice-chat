package com.alaydriem.bedrockvoicechat.api;

import com.alaydriem.bedrockvoicechat.config.ModConfig;

/**
 * Interface for platform-specific configuration loading.
 */
public interface ConfigProvider {
    /**
     * Load configuration from the platform-specific location.
     *
     * @return The loaded configuration
     */
    ModConfig load();

    /**
     * Save configuration to the platform-specific location.
     * Default implementation does nothing (not all platforms support saving).
     *
     * @param config The configuration to save
     */
    default void save(ModConfig config) {
        // Default: no-op
    }

    /**
     * Create the default configuration file if it doesn't exist.
     */
    void createDefaultIfMissing();
}
