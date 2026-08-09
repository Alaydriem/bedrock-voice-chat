package com.alaydriem.bedrockvoicechat.server

import com.alaydriem.bedrockvoicechat.config.generated.EmbeddedServerConfig
import com.google.gson.Gson
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNotEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class RuntimeConfigBuilderTest {
    private val gson = Gson()
    private val configDir = "/srv/bvc"

    private fun parse(json: String): EmbeddedServerConfig =
        gson.fromJson(json, EmbeddedServerConfig::class.java)

    @Test
    fun `fills the derived paths when the operator left them unset`() {
        val built = RuntimeConfigBuilder(configDir).build(EmbeddedServerConfig(), "token")

        assertEquals("$configDir/certificates", built.server?.tls?.certsPath)
        assertEquals("$configDir/assets", built.server?.assetsPath)
        assertEquals("$configDir/assets/audio", built.audio?.filePath)
        assertEquals("$configDir/bvc.sqlite3", built.database?.database)
    }

    @Test
    fun `keeps operator values instead of deriving over them`() {
        val source = parse(
            """
            {
              "server": { "assets_path": "/data/assets", "tls": { "certs_path": "/data/ca" } },
              "audio": { "file_path": "/data/clips" },
              "database": { "scheme": "postgres", "database": "bvc", "host": "db.internal" }
            }
            """.trimIndent()
        )

        val built = RuntimeConfigBuilder(configDir).build(source, "token")

        assertEquals("/data/ca", built.server?.tls?.certsPath)
        assertEquals("/data/assets", built.server?.assetsPath)
        assertEquals("/data/clips", built.audio?.filePath)
        assertEquals("bvc", built.database?.database)
        assertEquals("db.internal", built.database?.host)
    }

    // The sqlite path is a file in the data directory; a network database names
    // a schema on a host, so deriving a path over it would be wrong.
    @Test
    fun `does not derive a sqlite path for a network database`() {
        val source = parse("""{"database": {"scheme": "postgres", "host": "db.internal"}}""")

        val built = RuntimeConfigBuilder(configDir).build(source, "token")

        assertNull(built.database?.database)
    }

    @Test
    fun `derives the audio path from an operator supplied assets path`() {
        val source = parse("""{"server": {"assets_path": "/data/assets"}}""")

        val built = RuntimeConfigBuilder(configDir).build(source, "token")

        assertEquals("/data/assets/audio", built.audio?.filePath)
    }

    @Test
    fun `uses the configured access token`() {
        val builder = RuntimeConfigBuilder(configDir)
        val built = builder.build(EmbeddedServerConfig(), "configured-token")

        assertEquals("configured-token", built.server?.minecraft?.accessToken)
        assertEquals("configured-token", builder.resolvedAccessToken)
    }

    @Test
    fun `generates an access token when none is configured`() {
        val builder = RuntimeConfigBuilder(configDir)
        val built = builder.build(EmbeddedServerConfig(), "   ")

        val token = built.server?.minecraft?.accessToken
        assertTrue(!token.isNullOrBlank(), "a token must be generated")
        assertEquals(token, builder.resolvedAccessToken)
    }

    // listen and client_id are deliberately never sent: the server's own
    // defaults are correct and were being overridden with worse values.
    @Test
    fun `never sets listen or the minecraft client id`() {
        val built = RuntimeConfigBuilder(configDir).build(EmbeddedServerConfig(), "token")

        assertNull(built.server?.listen)
        assertNull(built.server?.minecraft?.clientId)
    }

    @Test
    fun `does not mutate the config the operator supplied`() {
        val source = EmbeddedServerConfig()
        RuntimeConfigBuilder(configDir).build(source, "token")

        assertNull(source.server)
    }

    @Test
    fun `two builds generate different tokens when none is configured`() {
        val first = RuntimeConfigBuilder(configDir).build(EmbeddedServerConfig(), null)
        val second = RuntimeConfigBuilder(configDir).build(EmbeddedServerConfig(), null)

        assertNotEquals(
            first.server?.minecraft?.accessToken,
            second.server?.minecraft?.accessToken
        )
    }
}
