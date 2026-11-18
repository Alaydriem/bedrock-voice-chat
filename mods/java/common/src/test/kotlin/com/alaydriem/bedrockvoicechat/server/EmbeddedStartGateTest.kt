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

    // An empty play block is complete: the registry supplies both the name and the
    // certificate, so there is nothing else for the operator to set.
    @Test
    fun `an empty play block allows a start`() {
        assertTrue(BvcServerManager.canStart(parse("""{"server": {"tls": {"play": {}}}}""")))
    }

    @Test
    fun `an enrollment token allows a start without a tls block`() {
        assertTrue(BvcServerManager.canStart(parse("""{"server": {"enrollment": {"token": "abc"}}}""")))
    }

    // A cleared token is no token, the same way the server reads it.
    @Test
    fun `a blank enrollment token refuses a start`() {
        assertFalse(BvcServerManager.canStart(parse("""{"server": {"enrollment": {"token": "  "}}}""")))
    }

    // The gate reads the server's resolved config, where a source set only through
    // BVC_* variables appears and unset string fields are empty rather than absent.
    @Test
    fun `a source resolved from environment variables allows a start`() {
        val enrolled = parse(
            """
            {"server": {
              "tls": {"certificate": "", "key": "", "certs_path": "/srv/bvc/certificates", "acme": null, "play": null},
              "enrollment": {"token": "from-env", "address": null, "port": null}
            }}
            """.trimIndent()
        )
        val manual = parse(
            """
            {"server": {"tls": {
              "certificate": "/certs/fullchain.pem", "key": "/certs/privkey.pem", "acme": null, "play": null
            }}}
            """.trimIndent()
        )
        val none = parse(
            """
            {"server": {
              "tls": {"certificate": "", "key": "", "acme": null, "play": null},
              "enrollment": {"token": null}
            }}
            """.trimIndent()
        )

        assertTrue(BvcServerManager.canStart(enrolled))
        assertTrue(BvcServerManager.canStart(manual))
        assertFalse(BvcServerManager.canStart(none))
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
