package com.alaydriem.bedrockvoicechat.paper

import org.bukkit.configuration.file.YamlConfiguration
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class PaperConfigProviderTest {

    private fun sectionOf(yaml: String) = YamlConfiguration().apply { loadFromString(yaml) }

    @Test
    fun `converts a nested yaml document to json`() {
        val yaml = """
            bvc-server: "https://bvc.example.com"
            access-token: "secret"
            minimum-players: 3
            use-embedded-server: true
            embedded-config:
              server:
                port: 8444
                tls:
                  certificate: "cert.pem"
                  names:
                    - "bvc.example.com"
                bedrock:
                  enabled: true
                  transfer_port: 19139
                  dns:
                    enabled: true
                    upstream:
                      - "1.1.1.1"
        """.trimIndent()

        val json = YamlSectionConverter.toJson(sectionOf(yaml))

        assertEquals("https://bvc.example.com", json.get("bvc-server").asString)
        assertEquals(3, json.get("minimum-players").asInt)

        val bedrock = json.getAsJsonObject("embedded-config")
            .getAsJsonObject("server")
            .getAsJsonObject("bedrock")
        assertEquals(true, bedrock.get("enabled").asBoolean)
        assertEquals(19139, bedrock.get("transfer_port").asInt)
        assertEquals(true, bedrock.getAsJsonObject("dns").get("enabled").asBoolean)
        assertEquals(
            "1.1.1.1",
            bedrock.getAsJsonObject("dns").getAsJsonArray("upstream").get(0).asString
        )
    }

    // The old reader parsed key by key and had no bedrock case at all, so the
    // whole block was silently dropped on Paper while Fabric honoured it.
    @Test
    fun `the bedrock block survives a full load`() {
        val yaml = """
            use-embedded-server: true
            embedded-config:
              server:
                bedrock:
                  enabled: true
                  transfer_port: 19139
        """.trimIndent()

        val config = PaperConfigProvider.fromJson(YamlSectionConverter.toJson(sectionOf(yaml)))

        val bedrock = config.embeddedConfig?.server?.bedrock
        assertEquals(true, bedrock?.enabled)
        assertEquals(19139, bedrock?.transferPort)
    }

    @Test
    fun `external mode keys still load`() {
        val yaml = """
            bvc-server: "https://bvc.example.com"
            access-token: "secret"
            minimum-players: 4
            use-embedded-server: false
        """.trimIndent()

        val config = PaperConfigProvider.fromJson(YamlSectionConverter.toJson(sectionOf(yaml)))

        assertEquals("https://bvc.example.com", config.bvcServer)
        assertEquals("secret", config.accessToken)
        assertEquals(4, config.minimumPlayers)
        assertEquals(false, config.useEmbeddedServer)
        assertTrue(config.isValid())
    }

    @Test
    fun `camel case spellings still load`() {
        val yaml = """
            bvcServer: "https://bvc.example.com"
            accessToken: "secret"
            minimumPlayers: 2
            useEmbeddedServer: false
        """.trimIndent()

        val config = PaperConfigProvider.fromJson(YamlSectionConverter.toJson(sectionOf(yaml)))

        assertEquals("https://bvc.example.com", config.bvcServer)
        assertEquals(2, config.minimumPlayers)
    }
}
