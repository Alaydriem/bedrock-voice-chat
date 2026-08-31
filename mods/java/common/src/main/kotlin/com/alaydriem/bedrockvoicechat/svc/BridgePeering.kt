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
 * It is also what makes a pairing code redeemable: the grant a redemption writes is
 * pinned to this key, and the session that follows presents the same one.
 *
 * `mint` is injected so the surrounding behaviour — the caching — is testable without
 * the cdylib. What the key itself guarantees (stable across reopens, parses back to this
 * node, unique per directory) is proven in `relay/sdk/tests/identity.rs`, against the
 * real implementation rather than a Kotlin restatement of it.
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
}
