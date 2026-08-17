package com.alaydriem.bedrockvoicechat.native

import org.slf4j.LoggerFactory
import java.io.File
import java.io.IOException
import java.security.MessageDigest

/**
 * Resolves a native library name to a verified file on disk.
 *
 * The digest is checked on every resolution, not only after a download. That is
 * what makes the cache directory untrusted storage: were it verified only at
 * download time, anything able to write that directory afterwards would have code
 * execution in the server process, and the check meant to prevent it would already
 * have passed.
 */
class NativeLibraryProvider(
    private val cacheRoot: File,
    private val manifest: NativeManifest,
    private val fetcher: LibraryFetcher,
    private val platform: NativePlatform = NativePlatform.current()
) {
    fun platformId(): String = platform.id

    fun cacheDirectory(): File = File(cacheRoot, "natives/${manifest.release}/${platform.id}")

    fun resolve(library: String): File {
        val entry = manifest.entry(library, platform)
        val target = File(cacheDirectory(), entry.asset)

        if (target.isFile && matches(target, entry.sha256)) {
            prune()
            return target
        }

        if (target.exists()) {
            logger.warn("Cached {} failed verification; discarding it", target.name)
            target.delete()
        }

        val bytes = sourceFor(library, entry).read()

        val actual = sha256(bytes)
        if (actual != entry.sha256) {
            throw NativeLibraryError.DigestMismatch(entry.sha256, actual)
        }

        write(target, bytes)

        // Verified again from disk rather than trusting the buffer that was just
        // written, so a truncated or partially flushed write is caught here rather
        // than by the loader.
        if (!matches(target, entry.sha256)) {
            target.delete()
            throw NativeLibraryError.DigestMismatch(entry.sha256, "unreadable after write")
        }

        prune()
        return target
    }

    /**
     * Resolves a library that will be loaded by bare name, and puts its directory
     * on JNA's search path.
     *
     * uniffi's generated bindings call `Native.load` with a bare name, so an
     * absolute path cannot be passed to them. This must run before the first
     * generated-binding call; afterwards the property is already read and the
     * failure is an UnsatisfiedLinkError with no obvious cause.
     */
    fun prepareForBareNameLoad(library: String): File {
        val resolved = resolve(library)
        val directory = resolved.parentFile.absolutePath

        val existing = System.getProperty(JNA_LIBRARY_PATH)
        val segments = existing?.split(File.pathSeparator)?.filter { it.isNotEmpty() } ?: emptyList()

        if (!segments.contains(directory)) {
            System.setProperty(JNA_LIBRARY_PATH, (segments + directory).joinToString(File.pathSeparator))
        }

        return resolved
    }

    private fun sourceFor(library: String, entry: NativeLibraryEntry): LibrarySource {
        val fileName = platform.fileNameFor(library)
        val bundled = "/native/${platform.id}/$fileName"

        return if (NativeLibraryProvider::class.java.getResource(bundled) != null) {
            LibrarySource.Bundled(platform, fileName)
        } else {
            LibrarySource.Remote(fetcher, manifest.assetUrl(entry))
        }
    }

    private fun write(target: File, bytes: ByteArray) {
        try {
            target.parentFile.mkdirs()
            // Written beside the target and moved, so an interrupted write never
            // leaves a partial file that the next start would verify and delete.
            val staging = File(target.parentFile, "${target.name}.partial")
            staging.writeBytes(bytes)
            if (!staging.renameTo(target)) {
                staging.copyTo(target, overwrite = true)
                staging.delete()
            }
        } catch (e: SecurityException) {
            throw NativeLibraryError.Write("permission_denied", "Cannot write ${target.absolutePath}", e)
        } catch (e: IOException) {
            val outcome = if (e.message?.contains("space", ignoreCase = true) == true) "no_space" else "io"
            throw NativeLibraryError.Write(outcome, "Cannot write ${target.absolutePath}", e)
        }
    }

    private fun matches(file: File, expected: String): Boolean =
        runCatching { sha256(file.readBytes()) == expected }.getOrDefault(false)

    /** Removes cache directories belonging to any release other than this one. */
    private fun prune() {
        val entries = File(cacheRoot, "natives").listFiles() ?: return
        for (dir in entries) {
            if (dir.isDirectory && dir.name != manifest.release) {
                dir.deleteRecursively()
            }
        }
    }

    companion object {
        private val logger = LoggerFactory.getLogger("BVC Native")

        private const val JNA_LIBRARY_PATH: String = "jna.library.path"

        fun sha256(bytes: ByteArray): String =
            MessageDigest.getInstance("SHA-256")
                .digest(bytes)
                .joinToString("") { "%02x".format(it) }
    }
}
