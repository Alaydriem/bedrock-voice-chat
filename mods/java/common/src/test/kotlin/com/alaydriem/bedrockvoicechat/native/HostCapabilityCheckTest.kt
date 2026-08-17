package com.alaydriem.bedrockvoicechat.native

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertNotNull
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.io.TempDir
import java.io.File

class HostCapabilityCheckTest {

    private val manifest = NativeManifest.parse(
        """
        {
          "release": "mods-v1.2.3",
          "base_url": "https://example.com/download/mods-v1.2.3",
          "libraries": {}
        }
        """.trimIndent()
    )

    private class StubFetcher(private val error: NativeLibraryError.Fetch?) : LibraryFetcher {
        var calls: Int = 0

        override fun fetch(url: String): ByteArray {
            calls += 1
            error?.let { throw it }
            return "{}".toByteArray()
        }
    }

    private fun provider(cache: File, fetcher: LibraryFetcher) =
        NativeLibraryProvider(cache, manifest, fetcher, NativePlatform.LINUX_X64)

    @Test
    fun `a host that can fetch and write reports ok for both`(@TempDir cache: File) {
        val fetcher = StubFetcher(null)

        val report = HostCapabilityCheck(
            provider(cache, fetcher), manifest, fetcher, "1.2.3", telemetryEnabled = true
        ).run()

        assertNotNull(report)
        assertEquals("ok", report!!.fetch)
        assertEquals("ok", report.write)
        assertEquals("linux-x64", report.platform)
        assertEquals("1.2.3", report.modVersion)
    }

    // The measurement this exists for: a reachable host that cannot write is not a
    // host that can run the skinny jar, and must not be recorded as one.
    @Test
    fun `a host that fetches but cannot write reports the write failure`(@TempDir cache: File) {
        val fetcher = StubFetcher(null)
        // A plain file where the cache directory needs to be makes mkdirs fail.
        File(cache, "natives").writeBytes(ByteArray(0))

        val report = HostCapabilityCheck(
            provider(cache, fetcher), manifest, fetcher, "1.2.3", telemetryEnabled = true
        ).run()

        assertEquals("ok", report!!.fetch)
        assertFalse(report.write == "ok", "write must not be reported as ok when it failed")
    }

    @Test
    fun `a fetch failure is reported by its class and write is not attempted`(@TempDir cache: File) {
        val fetcher = StubFetcher(NativeLibraryError.Fetch("timeout", "timed out"))

        val report = HostCapabilityCheck(
            provider(cache, fetcher), manifest, fetcher, "1.2.3", telemetryEnabled = true
        ).run()

        assertEquals("timeout", report!!.fetch)
        assertEquals("skipped", report.write)
    }

    // Telemetry off means no request and no file, not a result that is discarded.
    // An operator who turned telemetry off did not agree to the network request.
    @Test
    fun `with telemetry off nothing is fetched or written`(@TempDir cache: File) {
        val fetcher = StubFetcher(null)

        val report = HostCapabilityCheck(
            provider(cache, fetcher), manifest, fetcher, "1.2.3", telemetryEnabled = false
        ).run()

        assertNull(report)
        assertEquals(0, fetcher.calls)
        assertFalse(File(cache, "natives").exists())
    }
}
