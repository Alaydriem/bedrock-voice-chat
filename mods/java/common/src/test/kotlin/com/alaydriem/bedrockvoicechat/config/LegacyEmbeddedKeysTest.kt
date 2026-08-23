package com.alaydriem.bedrockvoicechat.config

import com.google.gson.JsonParser
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class LegacyEmbeddedKeysTest {

    private fun objectOf(json: String) = JsonParser.parseString(json).asJsonObject

    // Gson ignores unknown keys, so an upgraded config would otherwise boot with
    // none of the operator's settings and no message at all.
    @Test
    fun `detects a legacy flat key and names its replacement`() {
        val found = LegacyEmbeddedKeys.detect(objectOf("""{"http-port": 8444}"""))

        assertEquals(1, found.size)
        assertTrue(found[0].contains("http-port"), "found: ${found[0]}")
        assertTrue(found[0].contains("server.port"), "found: ${found[0]}")
    }

    @Test
    fun `detects every legacy key`() {
        val legacy = objectOf(
            """
            {
              "http-port": 8444, "quic-port": 8443, "broadcast-range": 32.0,
              "tls-certificate": "cert.pem", "tls-key": "key.pem",
              "tls-names": [], "tls-ips": [], "log-level": "info",
              "assets-path": "/data", "allow-audio-upload": true, "allow-audio-delete": true
            }
            """.trimIndent()
        )

        assertEquals(11, LegacyEmbeddedKeys.detect(legacy).size)
    }

    @Test
    fun `a nested config reports nothing`() {
        val nested = objectOf("""{"server": {"port": 8444, "tls": {"certificate": "cert.pem"}}}""")

        assertTrue(LegacyEmbeddedKeys.detect(nested).isEmpty())
    }

    @Test
    fun `a missing block reports nothing`() {
        assertTrue(LegacyEmbeddedKeys.detect(null).isEmpty())
    }

    // camelCase spellings were accepted as alternates, so an operator may have
    // either form on disk.
    @Test
    fun `detects the camel case spellings too`() {
        val found = LegacyEmbeddedKeys.detect(objectOf("""{"httpPort": 8444}"""))

        assertEquals(1, found.size)
        assertTrue(found[0].contains("server.port"), "found: ${found[0]}")
    }
}
