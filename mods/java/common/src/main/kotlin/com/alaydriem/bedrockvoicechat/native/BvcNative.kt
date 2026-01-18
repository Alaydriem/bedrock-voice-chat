package com.alaydriem.bedrockvoicechat.native

import com.sun.jna.Library
import com.sun.jna.Native
import com.sun.jna.Pointer
import org.slf4j.LoggerFactory
import java.io.File
import java.nio.file.Files

/**
 * JNA bindings for the native BVC server library.
 */
object BvcNative {
    private val logger = LoggerFactory.getLogger("BVC Native")
    private var library: BvcLibrary? = null

    /**
     * JNA interface for the native library.
     */
    interface BvcLibrary : Library {
        fun bvc_init(): Int
        fun bvc_server_create(configJson: String): Pointer?
        fun bvc_server_start(handle: Pointer): Int
        fun bvc_server_stop(handle: Pointer): Int
        fun bvc_server_destroy(handle: Pointer): Int
        fun bvc_update_positions(handle: Pointer, gameDataJson: String): Int
        fun bvc_get_last_error(): String?
        fun bvc_version(): String
    }

    /**
     * Load the native library from JAR resources.
     */
    @Synchronized
    fun load() {
        if (library != null) return

        val libName = getLibraryName()
        logger.info("Loading native library: {}", libName)

        try {
            // Try to extract from JAR resources
            val resourcePath = "/native/$libName"
            val resourceStream = BvcNative::class.java.getResourceAsStream(resourcePath)

            val libPath: String = if (resourceStream != null) {
                // Extract to temp directory
                val tempDir = Files.createTempDirectory("bvc-native").toFile()
                tempDir.deleteOnExit()

                val tempLib = File(tempDir, libName)
                tempLib.deleteOnExit()

                resourceStream.use { input ->
                    tempLib.outputStream().use { output ->
                        input.copyTo(output)
                    }
                }

                logger.info("Extracted native library to: {}", tempLib.absolutePath)
                tempLib.absolutePath
            } else {
                // Fall back to library name for system path lookup
                logger.info("Native library not found in JAR, trying system path")
                getLibraryBaseName()
            }

            library = Native.load(libPath, BvcLibrary::class.java)
            logger.info("Loaded native library successfully")

            // Initialize the crypto provider
            val initResult = library!!.bvc_init()
            if (initResult != 0) {
                logger.warn("Crypto provider init returned: {} (may already be initialized)", initResult)
            }
        } catch (e: Exception) {
            logger.error("Failed to load native library: {}", e.message, e)
            throw RuntimeException("Failed to load BVC native library", e)
        }
    }

    /**
     * Get the platform-specific library filename including architecture.
     * Library files are named: native/{os}-{arch}/lib_bvc_server.{ext}
     */
    private fun getLibraryName(): String {
        val os = System.getProperty("os.name").lowercase()
        val arch = getArchitecture()

        val osName = when {
            os.contains("win") -> "windows"
            os.contains("mac") || os.contains("darwin") -> "darwin"
            os.contains("linux") -> "linux"
            else -> throw UnsupportedOperationException("Unsupported OS: $os")
        }

        val ext = when {
            os.contains("win") -> "dll"
            os.contains("mac") || os.contains("darwin") -> "dylib"
            os.contains("linux") -> "so"
            else -> throw UnsupportedOperationException("Unsupported OS: $os")
        }

        // Library naming: lib_bvc_server.dll on Windows, liblib_bvc_server.so/dylib on Unix
        val libPrefix = if (os.contains("win")) "" else "lib"
        return "$osName-$arch/${libPrefix}lib_bvc_server.$ext"
    }

    /**
     * Get the CPU architecture (x64 or arm64).
     */
    private fun getArchitecture(): String {
        val arch = System.getProperty("os.arch").lowercase()
        return when {
            arch.contains("amd64") || arch.contains("x86_64") -> "x64"
            arch.contains("aarch64") || arch.contains("arm64") -> "arm64"
            else -> throw UnsupportedOperationException("Unsupported architecture: $arch")
        }
    }

    /**
     * Get the base name for system library loading.
     */
    private fun getLibraryBaseName(): String {
        return "lib_bvc_server"
    }

    private fun getLib(): BvcLibrary {
        load()
        return library ?: throw IllegalStateException("Native library not loaded")
    }

    /**
     * Create a server instance from JSON configuration.
     * @return handle on success, null on failure
     */
    fun createServer(configJson: String): Pointer? {
        val handle = getLib().bvc_server_create(configJson)
        if (handle == null) {
            logger.error("Failed to create server: {}", getLastError())
        }
        return handle
    }

    /**
     * Start the server. BLOCKS until shutdown.
     * Call from a dedicated thread.
     * @return 0 on success, -1 on error
     */
    fun startServer(handle: Pointer): Int {
        return getLib().bvc_server_start(handle)
    }

    /**
     * Signal the server to stop. Non-blocking, thread-safe.
     * @return 0 on success, -1 on error
     */
    fun stopServer(handle: Pointer): Int {
        return getLib().bvc_server_stop(handle)
    }

    /**
     * Destroy the server handle. Call after startServer returns.
     * @return 0 on success, -1 on error
     */
    fun destroyServer(handle: Pointer): Int {
        return getLib().bvc_server_destroy(handle)
    }

    /**
     * Update player positions directly via FFI (bypasses HTTP).
     * This is the preferred method for embedded mode.
     *
     * @param handle Server handle from createServer
     * @param gameDataJson JSON string containing game data with players:
     *   {"game": "minecraft", "players": [{"name": "Player1", "x": 100.0, ...}, ...]}
     * @return 0 on success, -1 on error
     */
    fun updatePositions(handle: Pointer, gameDataJson: String): Int {
        val result = getLib().bvc_update_positions(handle, gameDataJson)
        if (result != 0) {
            logger.warn("Failed to update positions: {}", getLastError())
        }
        return result
    }

    /**
     * Get the last error message from the native library.
     */
    fun getLastError(): String? {
        return library?.bvc_get_last_error()
    }

    /**
     * Get the native library version.
     */
    fun getVersion(): String {
        return getLib().bvc_version()
    }
}
