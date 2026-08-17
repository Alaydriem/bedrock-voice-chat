package com.alaydriem.bedrockvoicechat.native

/**
 * The seam between resolution and the network.
 *
 * Tests substitute this rather than reaching GitHub: asserting that GitHub serves
 * files tests GitHub.
 */
interface LibraryFetcher {
    /**
     * Returns the body, or throws [NativeLibraryError.Fetch] carrying the outcome
     * value that classifies the failure.
     */
    fun fetch(url: String): ByteArray
}
