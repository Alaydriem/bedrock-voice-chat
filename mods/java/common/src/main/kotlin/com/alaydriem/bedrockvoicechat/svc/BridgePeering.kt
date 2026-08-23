package com.alaydriem.bedrockvoicechat.svc

import uniffi.bvc_relay_sdk.BvcIdentity
import java.io.File

/**
 * This bridge's own identity on the peer plane.
 *
 * The node key is minted and persisted before any session is opened, which is what
 * lets the far side grant this bridge before the bridge connects. Deriving the link
 * from an open session instead would deadlock the embedded case: opening one needs
 * the server's link, and the server cannot grant us until it has ours.
 *
 * `mint` is injected so the surrounding behaviour — caching, and the block an
 * operator pastes — is testable without the cdylib. What the key itself guarantees
 * (stable across reopens, parses back to this node, unique per directory) is proven
 * in `relay/sdk/tests/identity.rs`, against the real implementation rather than a
 * Kotlin restatement of it.
 */
class BridgePeering(
    private val nodeDir: File,
    private val mint: (String) -> String = { dir -> BvcIdentity.open(dir).use { it.peerlink() } }
) {
    @Volatile
    private var cached: String? = null

    @Synchronized
    fun peerlink(): String {
        cached?.let { return it }

        nodeDir.mkdirs()
        return mint(nodeDir.absolutePath).also { cached = it }
    }

    /**
     * The block an operator pastes into the `server` block of the BVC server's
     * `config.hcl`.
     *
     * `peers`, not `peer`. hcl-rs names the resulting map after the block identifier
     * verbatim and the field it has to reach is `Server::peers`, so a `peer` block is
     * valid HCL that lands in a key nothing reads — the server goes on reporting
     * "peering is not configured" with the grant sitting in the file. It is nested
     * under `server` for the same reason: that is where the field lives.
     *
     * `capabilities` is omitted deliberately: it defaults to `carry_speakers`, which
     * is exactly what a voice bridge needs, and naming it here would invite someone
     * to widen it by copying.
     */
    fun grantBlock(): String = """
        peers "svc-bridge" {
          peerlink = "${peerlink()}"
        }
    """.trimIndent()
}
