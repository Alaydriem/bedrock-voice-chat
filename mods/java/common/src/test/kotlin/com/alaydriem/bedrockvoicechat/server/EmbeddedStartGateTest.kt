package com.alaydriem.bedrockvoicechat.server

import com.alaydriem.bedrockvoicechat.config.generated.EmbeddedServerConfig
import com.google.gson.Gson
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class EmbeddedStartGateTest {
    private val gson = Gson()

    private fun parse(json: String): EmbeddedServerConfig =
        gson.fromJson(json, EmbeddedServerConfig::class.java)

    @Test
    fun `manual certificate and key allow a start`() {
        val config = parse(
            """{"server": {"tls": {"certificate": "cert.pem", "key": "key.pem"}}}"""
        )

        assertTrue(BvcServerManager.canStart(config))
    }

    // The runtime obtains a certificate over DNS-01 itself, so requiring a
    // manual one would make an implemented feature unreachable.
    @Test
    fun `an acme block allows a start without a manual certificate`() {
        val config = parse(
            """
            {"server": {"tls": {"acme": {
              "email": "ops@example.com", "provider": "cloudflare", "api_token": "token"
            }}}}
            """.trimIndent()
        )

        assertTrue(BvcServerManager.canStart(config))
    }

    @Test
    fun `neither certificates nor acme refuses a start`() {
        assertFalse(BvcServerManager.canStart(parse("""{"server": {"tls": {}}}""")))
        assertFalse(BvcServerManager.canStart(EmbeddedServerConfig()))
        assertFalse(BvcServerManager.canStart(null))
    }

    @Test
    fun `a certificate without its key refuses a start`() {
        val config = parse("""{"server": {"tls": {"certificate": "cert.pem"}}}""")

        assertFalse(BvcServerManager.canStart(config))
    }
}
