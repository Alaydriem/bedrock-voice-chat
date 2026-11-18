package com.alaydriem.bedrockvoicechat.powernukkitx

import com.alaydriem.bedrockvoicechat.config.ModConfig
import com.alaydriem.bedrockvoicechat.server.BvcServerManager
import com.alaydriem.bedrockvoicechat.server.RuntimeConfigBuilder
import com.google.gson.Gson
import com.google.gson.JsonObject
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNotNull
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.io.TempDir
import org.powernukkitx.utils.Config
import java.io.File

class PnxConfigProviderTest {

    private fun load(dataFolder: File, yaml: String): ModConfig {
        val file = File(dataFolder, "config.yml").apply { writeText(yaml) }
        return PnxConfigProvider.fromConfig(Config(file, Config.YAML))
    }

    // PowerNukkitX turns nested maps into its own sections; an empty one that went
    // missing on the way would switch play mode off with nothing in the log.
    @Test
    fun an_empty_play_block_reaches_the_embedded_server(@TempDir dataFolder: File) {
        val config = load(
            dataFolder,
            """
            use-embedded-server: true
            embedded-config:
              server:
                port: 8444
                tls:
                  play: {}
            """.trimIndent(),
        )

        val embedded = config.embeddedConfig
        assertNotNull(embedded?.server?.tls?.play)
        assertTrue(BvcServerManager.canStart(embedded))

        val builder = RuntimeConfigBuilder(dataFolder.absolutePath)
        val json = Gson().fromJson(builder.toJson(builder.build(embedded, "token")), JsonObject::class.java)
        assertTrue(json.getAsJsonObject("server").getAsJsonObject("tls").get("play").isJsonObject)
    }

    @Test
    fun a_play_address_is_kept(@TempDir dataFolder: File) {
        val config = load(
            dataFolder,
            """
            use-embedded-server: true
            embedded-config:
              server:
                tls:
                  play:
                    address: "203.0.113.10"
            """.trimIndent(),
        )

        assertEquals("203.0.113.10", config.embeddedConfig?.server?.tls?.play?.address)
    }

    @Test
    fun the_shipped_config_loads_in_external_mode(@TempDir dataFolder: File) {
        val shipped = javaClass.classLoader.getResource("config.yml")!!.readText()

        val config = load(dataFolder, shipped)

        assertEquals(false, config.useEmbeddedServer)
        assertEquals(1, config.minimumPlayers)
    }
}
