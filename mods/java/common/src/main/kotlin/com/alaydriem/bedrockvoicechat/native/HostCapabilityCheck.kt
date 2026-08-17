package com.alaydriem.bedrockvoicechat.native

import org.slf4j.LoggerFactory
import java.io.File
import java.io.IOException

/**
 * Measures whether this host could run the skinny jar.
 *
 * Running the skinny jar takes two capabilities — fetching a library and writing
 * it to disk — so both are measured. A host that can fetch but cannot write its
 * cache directory cannot run it either, and reachability alone would score that
 * host as ready.
 *
 * The check is the real resolution path, fetch then write, at a few kilobytes
 * rather than about twelve megabytes. Its result changes nothing about the run: a
 * failure is data, and is never surfaced to the operator.
 */
class HostCapabilityCheck(
    private val provider: NativeLibraryProvider,
    private val manifest: NativeManifest,
    private val fetcher: LibraryFetcher,
    private val modVersion: String,
    private val telemetryEnabled: Boolean
) {
    /**
     * Returns the report, or null when telemetry is off — in which case no request
     * is made and nothing is written.
     */
    fun run(): HostCapabilityReport? {
        if (!telemetryEnabled) {
            return null
        }

        val variant = if (isBundled()) "fat" else "skinny"

        val body = try {
            fetcher.fetch(manifest.manifestUrl())
        } catch (e: NativeLibraryError.Fetch) {
            logger.debug("Host capability check could not fetch: {}", e.outcome)
            return HostCapabilityReport(variant, provider.platformId(), modVersion, e.outcome, "skipped")
        }

        return HostCapabilityReport(variant, provider.platformId(), modVersion, "ok", writeOutcome(body))
    }

    /**
     * The written copy is kept as the cache directory's own manifest rather than
     * deleted, so the check leaves a real artifact and needs no cleanup step that
     * could itself fail.
     */
    private fun writeOutcome(body: ByteArray): String = try {
        val target = File(provider.cacheDirectory(), NativeManifest.MANIFEST_NAME)
        target.parentFile.mkdirs()
        target.writeBytes(body)
        if (target.isFile) "ok" else "io"
    } catch (e: SecurityException) {
        logger.debug("Host capability check could not write: {}", e.toString())
        "permission_denied"
    } catch (e: IOException) {
        logger.debug("Host capability check could not write: {}", e.toString())
        if (e.message?.contains("space", ignoreCase = true) == true) "no_space" else "io"
    }

    private fun isBundled(): Boolean =
        HostCapabilityCheck::class.java.getResource("/native/${provider.platformId()}") != null

    companion object {
        private val logger = LoggerFactory.getLogger("BVC Native")
    }
}
