package com.alaydriem.bedrockvoicechat.native

import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.io.TempDir
import java.io.File

class RelaySdkLoadPathTest {

    private val payload = "relay sdk bytes".toByteArray()

    private class FixedFetcher(private val body: ByteArray) : LibraryFetcher {
        override fun fetch(url: String): ByteArray = body
    }

    private fun manifest() = NativeManifest.parse(
        """
        {
          "release": "mods-v1.2.3",
          "base_url": "https://example.com/download/mods-v1.2.3",
          "libraries": {
            "bvc_relay_sdk": {
              "linux-x64": {
                "asset": "libbvc_relay_sdk-linux-x64.so",
                "sha256": "${NativeLibraryProvider.sha256(payload)}"
              }
            }
          }
        }
        """.trimIndent()
    )

    // uniffi's generated bindings call Native.load with a bare name, so the cache
    // directory has to be on JNA's search path before the first binding call.
    // Nothing about that ordering is visible to the compiler: getting it wrong
    // surfaces only at runtime, as an UnsatisfiedLinkError with no obvious cause.
    @Test
    fun `preparing a bare name load puts the cache directory on the jna search path`(@TempDir cache: File) {
        val provider = NativeLibraryProvider(cache, manifest(), FixedFetcher(payload), NativePlatform.LINUX_X64)

        provider.prepareForBareNameLoad("bvc_relay_sdk")

        val searchPath = System.getProperty("jna.library.path") ?: ""
        assertTrue(
            searchPath.split(File.pathSeparator).contains(provider.cacheDirectory().absolutePath),
            "jna.library.path must contain ${provider.cacheDirectory().absolutePath}, was: $searchPath"
        )
    }

    // Repeated preparation must not grow the property without bound: a server that
    // reloads the plugin would otherwise accumulate a segment per reload.
    @Test
    fun `preparing twice does not duplicate the entry`(@TempDir cache: File) {
        val provider = NativeLibraryProvider(cache, manifest(), FixedFetcher(payload), NativePlatform.LINUX_X64)

        provider.prepareForBareNameLoad("bvc_relay_sdk")
        provider.prepareForBareNameLoad("bvc_relay_sdk")

        val occurrences = (System.getProperty("jna.library.path") ?: "")
            .split(File.pathSeparator)
            .count { it == provider.cacheDirectory().absolutePath }

        assertTrue(occurrences == 1, "expected exactly one entry, found $occurrences")
    }
}
