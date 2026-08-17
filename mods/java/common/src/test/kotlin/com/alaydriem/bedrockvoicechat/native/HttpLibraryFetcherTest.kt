package com.alaydriem.bedrockvoicechat.native

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class HttpLibraryFetcherTest {

    // A plain-HTTP URL must be refused before any request is made. The pinned
    // digest is the trust root, but a downgrade would still expose the fetch to
    // an interception the operator never opted into.
    @Test
    fun `refuses a non https url without making a request`() {
        val error = assertThrows(NativeLibraryError.Fetch::class.java) {
            HttpLibraryFetcher().fetch("http://example.invalid/native-manifest.json")
        }
        assertEquals("tls", error.outcome)
    }

    // .invalid is reserved by RFC 2606 and never resolves, so this asserts the
    // classifier without depending on any real host being down. A resolver that
    // answers wildcards for unresolvable names turns this into a connect failure
    // instead, which is why both are accepted.
    @Test
    fun `classifies an unreachable host as a connection level outcome`() {
        val error = assertThrows(NativeLibraryError.Fetch::class.java) {
            HttpLibraryFetcher().fetch("https://nonexistent.invalid/native-manifest.json")
        }
        assertTrue(
            error.outcome in setOf("dns", "refused", "timeout", "io"),
            "expected a connection-level outcome, got ${error.outcome}"
        )
    }
}
