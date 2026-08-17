package com.alaydriem.bedrockvoicechat.svc

import com.alaydriem.bedrockvoicechat.config.generated.EmbeddedServerConfig
import com.alaydriem.bedrockvoicechat.config.generated.PeerConfig
import com.alaydriem.bedrockvoicechat.config.generated.Server
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.io.TempDir
import java.io.File

class EmbeddedGrantTest {

    private fun grant(dir: File) = EmbeddedGrant(BridgePeering(dir) { "bvcpeerAAAA" })

    @Test
    fun `the bridge is granted on a config that had no peers`(@TempDir dir: File) {
        val config = EmbeddedServerConfig()

        grant(dir).applyTo(config)

        assertEquals("bvcpeerAAAA", config.server!!.peers!!["svc-bridge"]!!.peerlink)
    }

    // An operator who declared other peers keeps them. Replacing the map would
    // silently revoke every server they had paired.
    @Test
    fun `an existing peer survives the grant`(@TempDir dir: File) {
        val config = EmbeddedServerConfig().apply {
            server = Server().apply {
                peers = mapOf("other-server" to PeerConfig().apply { peerlink = "bvcpeerBBBB" })
            }
        }

        grant(dir).applyTo(config)

        assertEquals(2, config.server!!.peers!!.size)
        assertEquals("bvcpeerBBBB", config.server!!.peers!!["other-server"]!!.peerlink)
    }

    // Left unset so the bridge declares its worlds at handshake. Pinning them here
    // would narrow the grant to a value the operator never chose, and the symptom
    // is audio dropped for a world that looks correctly configured.
    @Test
    fun `the grant pins neither worlds nor capabilities`(@TempDir dir: File) {
        val config = EmbeddedServerConfig()

        grant(dir).applyTo(config)

        val peer = config.server!!.peers!!["svc-bridge"]!!
        assertNull(peer.worlds)
        assertNull(peer.capabilities)
    }

    // Re-granting on a restart must not accumulate entries or change the key.
    @Test
    fun `granting twice leaves one entry`(@TempDir dir: File) {
        val config = EmbeddedServerConfig()
        val grant = grant(dir)

        grant.applyTo(config)
        grant.applyTo(config)

        assertEquals(1, config.server!!.peers!!.size)
        assertTrue(config.server!!.peers!!.containsKey("svc-bridge"))
    }
}
