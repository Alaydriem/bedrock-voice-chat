package com.alaydriem.bedrockvoicechat.config

import com.alaydriem.bedrockvoicechat.config.generated.EmbeddedServerConfig
import com.alaydriem.bedrockvoicechat.config.generated.Server
import com.google.gson.Gson
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Test

class GeneratedConfigTest {
    private val gson = Gson()

    @Test
    fun `nested operator config deserializes into the generated classes`() {
        val json = """
            {
              "server": {
                "port": 8444,
                "tls": { "certificate": "cert.pem", "names": ["bvc.example.com"] },
                "bedrock": { "enabled": true, "transfer_port": 19139 }
              },
              "voice": { "spatial_audio": { "broadcast_range": 32.0 } }
            }
        """.trimIndent()

        val config = gson.fromJson(json, EmbeddedServerConfig::class.java)

        assertEquals(8444L, config.server?.port)
        assertEquals("cert.pem", config.server?.tls?.certificate)
        assertEquals(listOf("bvc.example.com"), config.server?.tls?.names)
        assertEquals(19139, config.server?.bedrock?.transferPort)
        assertEquals(32.0f, config.voice?.spatialAudio?.broadcastRange)
    }

    // broadcast-range was the defect that started this work: the mod sent it at
    // voice.broadcast_range, which no server field matches, so serde dropped it
    // and every embedded server ran at the default.
    @Test
    fun `broadcast range sits under voice spatial_audio`() {
        val config = EmbeddedServerConfig().apply {
            voice = com.alaydriem.bedrockvoicechat.config.generated.Voice().apply {
                spatialAudio =
                    com.alaydriem.bedrockvoicechat.config.generated.SpatialAudioConfig().apply {
                        broadcastRange = 32.0f
                    }
            }
        }

        val rendered = gson.toJson(config)

        assertEquals("""{"voice":{"spatial_audio":{"broadcast_range":32.0}}}""", rendered)
    }

    // An unset key must be absent from the JSON entirely so the server's own
    // serde default applies. A serialized null would not do that.
    @Test
    fun `unset fields are omitted from the serialized json`() {
        val rendered = gson.toJson(EmbeddedServerConfig())

        assertEquals("{}", rendered)
    }

    @Test
    fun `carved out sections have no generated field`() {
        val fields = Server::class.java.declaredFields.map { it.name }

        assertFalse(fields.contains("meridian"), "server.meridian must be carved out")
        assertFalse(fields.contains("cors"), "server.cors must be carved out")
    }

    // Gson discards what it does not recognise, which is exactly why the mod
    // needs its own guard for keys an operator may still have on disk.
    @Test
    fun `an unknown key is ignored rather than failing the parse`() {
        val config = gson.fromJson(
            """{"server": {"meridian": {"url": "https://example.com"}, "port": 8444}}""",
            EmbeddedServerConfig::class.java
        )

        assertEquals(8444L, config.server?.port)
        assertNull(config.server?.listen)
    }
}
