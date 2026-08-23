package com.alaydriem.bedrockvoicechat.svc

import com.alaydriem.bedrockvoicechat.config.generated.EmbeddedServerConfig
import com.alaydriem.bedrockvoicechat.config.generated.PeerConfig
import com.alaydriem.bedrockvoicechat.config.generated.Server

/**
 * Grants this bridge on the embedded server it is about to start.
 *
 * The embedded case owns both sides, so the operator should not have to copy a
 * peerlink between two things the mod already controls. What is written is the same
 * `peer` block an external operator writes by hand — a declaration they can read and
 * delete — rather than a runtime enrolment route the external path lacks.
 *
 * Applied before the server starts, because authorization is read from config at
 * startup and revocation requires a restart.
 */
class EmbeddedGrant(private val peering: BridgePeering) {

    fun applyTo(config: EmbeddedServerConfig) {
        val server = config.server ?: Server().also { config.server = it }

        val peer = PeerConfig().apply {
            peerlink = peering.peerlink()
        }

        // `worlds` is left unset so the bridge declares its own at handshake, and
        // `capabilities` defaults to carry_speakers. Naming either here would pin a
        // value the operator did not choose.
        server.peers = (server.peers ?: emptyMap()) + (LABEL to peer)
    }

    companion object {
        const val LABEL: String = "svc-bridge"
    }
}
