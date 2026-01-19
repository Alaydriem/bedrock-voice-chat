package com.alaydriem.bedrockvoicechat.api

import com.alaydriem.bedrockvoicechat.config.ModConfig
import java.nio.file.Path

/**
 * Interface for platform-specific configuration loading.
 */
interface ConfigProvider {
    /**
     * Load configuration from the platform-specific location.
     */
    fun load(): ModConfig

    /**
     * Save configuration to the platform-specific location.
     * Default implementation does nothing (not all platforms support saving).
     */
    fun save(config: ModConfig) {
    }

    /**
     * Create the default configuration file if it doesn't exist.
     */
    fun createDefaultIfMissing()

    /**
     * Get the configuration directory path for this platform.
     * Used for embedded server mode to store certificates and database.
     * Returns null if the platform doesn't support a config directory.
     */
    fun getConfigDir(): Path?
}
