package com.alaydriem.bedrockvoicechat.svc

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.io.TempDir
import java.io.File

/**
 * What the key itself guarantees — stable across reopens, parses back to this node,
 * unique per directory — is proven against the real implementation in
 * `relay/sdk/tests/identity.rs`. These cover what Kotlin adds on top.
 */
class BridgePeeringTest {

    @Test
    fun `the grant block names the bridge and its peerlink`(@TempDir dir: File) {
        val peering = BridgePeering(dir) { "bvcpeerAAAA" }

        val block = peering.grantBlock()

        assertTrue(block.contains("peer \"svc-bridge\""))
        assertTrue(block.contains("bvcpeerAAAA"))
    }

    // The block is pasted into an HCL file, so it has to be a `peer` block with a
    // quoted label and a quoted value — not a fragment an operator has to repair.
    @Test
    fun `the grant block is shaped like the hcl it is pasted into`(@TempDir dir: File) {
        val block = BridgePeering(dir) { "bvcpeerAAAA" }.grantBlock()

        assertEquals(
            """
            peer "svc-bridge" {
              peerlink = "bvcpeerAAAA"
            }
            """.trimIndent(),
            block
        )
    }

    // Minting reads and may write a key file. Doing it per frame, or per call from
    // a startup path that asks twice, is work for a value that cannot change.
    @Test
    fun `the key is minted once however often it is asked for`(@TempDir dir: File) {
        var mints = 0
        val peering = BridgePeering(dir) { mints += 1; "bvcpeerAAAA" }

        peering.peerlink()
        peering.peerlink()
        peering.grantBlock()

        assertEquals(1, mints)
    }

    // The SDK creates the key file, and it cannot do that in a directory that does
    // not exist. A first start has no node directory yet.
    @Test
    fun `the node directory exists before the key is minted`(@TempDir parent: File) {
        val nodeDir = File(parent, "node")
        var existedAtMint = false

        BridgePeering(nodeDir) { existedAtMint = nodeDir.isDirectory; "bvcpeerAAAA" }.peerlink()

        assertTrue(existedAtMint)
    }
}
