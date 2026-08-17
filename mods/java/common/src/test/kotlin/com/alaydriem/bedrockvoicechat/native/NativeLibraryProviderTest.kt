package com.alaydriem.bedrockvoicechat.native

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.io.TempDir
import java.io.File

class NativeLibraryProviderTest {

    private val payload = "native library bytes".toByteArray()
    private val digest = NativeLibraryProvider.sha256(payload)

    private class RecordingFetcher(private val body: ByteArray?) : LibraryFetcher {
        var calls: Int = 0

        override fun fetch(url: String): ByteArray {
            calls += 1
            return body ?: throw NativeLibraryError.Fetch("timeout", "no body configured")
        }
    }

    private fun manifest(sha: String) = NativeManifest.parse(
        """
        {
          "release": "mods-v1.2.3",
          "base_url": "https://example.com/download/mods-v1.2.3",
          "libraries": {
            "bvc_relay_sdk": {
              "linux-x64": { "asset": "libbvc_relay_sdk-linux-x64.so", "sha256": "$sha" }
            }
          }
        }
        """.trimIndent()
    )

    @Test
    fun `fetches writes and returns a verified library`(@TempDir cache: File) {
        val fetcher = RecordingFetcher(payload)
        val provider = NativeLibraryProvider(cache, manifest(digest), fetcher, NativePlatform.LINUX_X64)

        val resolved = provider.resolve("bvc_relay_sdk")

        assertTrue(resolved.exists())
        assertEquals(1, fetcher.calls)
        assertEquals(payload.toList(), resolved.readBytes().toList())
    }

    // The second resolution must not touch the network. This is what makes a
    // restart free rather than a second download.
    @Test
    fun `a cache hit performs no fetch`(@TempDir cache: File) {
        val fetcher = RecordingFetcher(payload)
        val manifest = manifest(digest)

        NativeLibraryProvider(cache, manifest, fetcher, NativePlatform.LINUX_X64).resolve("bvc_relay_sdk")
        NativeLibraryProvider(cache, manifest, fetcher, NativePlatform.LINUX_X64).resolve("bvc_relay_sdk")

        assertEquals(1, fetcher.calls)
    }

    // The security property of the whole design: a cached file that no longer
    // matches must never be loaded, however it came to differ.
    @Test
    fun `a tampered cached library is refused and deleted`(@TempDir cache: File) {
        val fetcher = RecordingFetcher(payload)
        val manifest = manifest(digest)
        val resolved = NativeLibraryProvider(cache, manifest, fetcher, NativePlatform.LINUX_X64)
            .resolve("bvc_relay_sdk")

        resolved.writeBytes("tampered".toByteArray())

        val offline = RecordingFetcher(null)
        assertThrows(NativeLibraryError.Fetch::class.java) {
            NativeLibraryProvider(cache, manifest, offline, NativePlatform.LINUX_X64).resolve("bvc_relay_sdk")
        }

        assertFalse(resolved.exists(), "a file that failed verification must not be left behind")
    }

    @Test
    fun `fetched bytes that do not match the pinned digest are refused`(@TempDir cache: File) {
        val fetcher = RecordingFetcher(payload)
        val wrong = manifest("0000000000000000000000000000000000000000000000000000000000000000")

        val error = assertThrows(NativeLibraryError.DigestMismatch::class.java) {
            NativeLibraryProvider(cache, wrong, fetcher, NativePlatform.LINUX_X64).resolve("bvc_relay_sdk")
        }

        assertTrue(error.message!!.contains("0000000000000000"))
    }

    // An upgrade must not leave the previous release's libraries on disk forever.
    @Test
    fun `resolving prunes a previous release cache directory`(@TempDir cache: File) {
        val stale = File(cache, "natives/mods-v1.0.0/linux-x64").apply { mkdirs() }
        File(stale, "libbvc_relay_sdk-linux-x64.so").writeBytes("old".toByteArray())

        NativeLibraryProvider(cache, manifest(digest), RecordingFetcher(payload), NativePlatform.LINUX_X64)
            .resolve("bvc_relay_sdk")

        assertFalse(File(cache, "natives/mods-v1.0.0").exists())
        assertTrue(File(cache, "natives/mods-v1.2.3").exists())
    }
}
