package com.alaydriem.bedrockvoicechat.native

import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.io.TempDir
import java.io.File

class DevBuildDiagnosticTest {

    private class NeverCalledFetcher : LibraryFetcher {
        var calls: Int = 0

        override fun fetch(url: String): ByteArray {
            calls += 1
            throw NativeLibraryError.Fetch("http_404", "Unexpected status 404 for $url")
        }
    }

    private fun manifest(release: String) = NativeManifest.parse(
        """
        {
          "release": "$release",
          "base_url": "https://example.com/download/$release",
          "libraries": {
            "bvc_relay_sdk": {
              "linux-x64": { "asset": "libbvc_relay_sdk-linux-x64.so", "sha256": "aaaa" }
            }
          }
        }
        """.trimIndent()
    )

    // A locally built skinny jar pins no release, so every asset URL it can build is
    // one that was never published. Reporting the 404 sends the reader to look at a
    // release that does not exist instead of at the jar they built.
    @Test
    fun `a local build says it is a local build rather than reporting a 404`(@TempDir cache: File) {
        val fetcher = NeverCalledFetcher()
        val provider = NativeLibraryProvider(cache, manifest("dev"), fetcher, NativePlatform.LINUX_X64)

        val error = assertThrows(NativeLibraryError.Fetch::class.java) {
            provider.resolve("bvc_relay_sdk")
        }

        assertTrue(
            error.message!!.contains("-Pbundled"),
            "the message must name the fix, was: ${error.message}"
        )
        assertTrue(fetcher.calls == 0, "a local build must not attempt a download at all")
    }

    // A real release still fetches. The diagnostic above must not swallow the
    // ordinary path.
    @Test
    fun `a pinned release still attempts the download`(@TempDir cache: File) {
        val fetcher = NeverCalledFetcher()
        val provider =
            NativeLibraryProvider(cache, manifest("mods-v1.2.3"), fetcher, NativePlatform.LINUX_X64)

        assertThrows(NativeLibraryError.Fetch::class.java) {
            provider.resolve("bvc_relay_sdk")
        }

        assertTrue(fetcher.calls == 1, "a pinned release must still be fetched")
    }
}
