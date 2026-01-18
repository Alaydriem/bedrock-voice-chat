package com.alaydriem.bedrockvoicechat.native

import org.slf4j.LoggerFactory
import java.io.File
import java.nio.file.Files

/**
 * JNI bindings for the native BVC server library.
 */
object BvcNative {
    private val logger = LoggerFactory.getLogger("BVC Native")
    private var loaded = false

    /**
     * Load the native library from JAR resources.
     * Extracts the platform-specific library to a temp directory and loads it.
     */
    @Synchronized
    fun load() {
        if (loaded) return

        val libName = getLibraryName()
        logger.info("Loading native library: {}", libName)

        try {
            // Try to extract from JAR resources
            val resourcePath = "/native/$libName"
            val resourceStream = BvcNative::class.java.getResourceAsStream(resourcePath)

            if (resourceStream != null) {
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

                System.load(tempLib.absolutePath)
                logger.info("Loaded native library from JAR: {}", tempLib.absolutePath)
            } else {
                // Fall back to system library path
                val baseName = libName.removeSuffix(".dll").removeSuffix(".so").removeSuffix(".dylib")
                    .removePrefix("lib")
                System.loadLibrary(baseName)
                logger.info("Loaded native library from system: {}", baseName)
            }

            loaded = true

            // Initialize the crypto provider
            val initResult = nativeInit()
            if (initResult != 0) {
                logger.warn("Crypto provider initialization returned: {} (may already be initialized)", initResult)
            }
        } catch (e: Exception) {
            logger.error("Failed to load native library: {}", e.message, e)
            throw RuntimeException("Failed to load BVC native library", e)
        }
    }

    /**
     * Get the platform-specific library filename.
     */
    private fun getLibraryName(): String {
        val os = System.getProperty("os.name").lowercase()
        val arch = System.getProperty("os.arch").lowercase()

        val archSuffix = when {
            arch.contains("aarch64") || arch.contains("arm64") -> "arm64"
            arch.contains("amd64") || arch.contains("x86_64") -> "x64"
            else -> arch
        }

        return when {
            os.contains("win") -> "lib_bvc_server.dll"
            os.contains("mac") || os.contains("darwin") -> "liblib_bvc_server.dylib"
            os.contains("linux") -> "liblib_bvc_server.so"
            else -> throw UnsupportedOperationException("Unsupported OS: $os")
        }
    }

    // Native method declarations

    /**
     * Initialize the crypto provider. Must be called before creating servers.
     * Returns 0 on success, -1 on error.
     */
    @JvmStatic
    private external fun nativeInit(): Int

    /**
     * Create a server instance from JSON configuration.
     * Returns a handle (pointer) on success, 0 on error.
     */
    @JvmStatic
    private external fun nativeCreateServer(configJson: String): Long

    /**
     * Start the server. BLOCKS until shutdown.
     * Returns 0 on clean shutdown, -1 on error.
     */
    @JvmStatic
    private external fun nativeStartServer(handle: Long): Int

    /**
     * Signal the server to stop gracefully. Non-blocking.
     * Returns 0 on success, -1 on error.
     */
    @JvmStatic
    private external fun nativeStopServer(handle: Long): Int

    /**
     * Destroy the server handle and free resources.
     * Returns 0 on success, -1 on error.
     */
    @JvmStatic
    private external fun nativeDestroyServer(handle: Long): Int

    /**
     * Get the last error message.
     * Returns null if no error.
     */
    @JvmStatic
    private external fun nativeGetLastError(): String?

    /**
     * Get the library version.
     */
    @JvmStatic
    private external fun nativeGetVersion(): String

    // Public API wrappers

    /**
     * Create a server instance from JSON configuration.
     * @return handle on success, 0 on failure
     */
    fun createServer(configJson: String): Long {
        load()
        return nativeCreateServer(configJson)
    }

    /**
     * Start the server. BLOCKS until shutdown.
     * Call from a dedicated thread.
     * @return 0 on success, -1 on error
     */
    fun startServer(handle: Long): Int {
        return nativeStartServer(handle)
    }

    /**
     * Signal the server to stop. Non-blocking, thread-safe.
     * @return 0 on success, -1 on error
     */
    fun stopServer(handle: Long): Int {
        return nativeStopServer(handle)
    }

    /**
     * Destroy the server handle. Call after startServer returns.
     * @return 0 on success, -1 on error
     */
    fun destroyServer(handle: Long): Int {
        return nativeDestroyServer(handle)
    }

    /**
     * Get the last error message from the native library.
     */
    fun getLastError(): String? {
        return nativeGetLastError()
    }

    /**
     * Get the native library version.
     */
    fun getVersion(): String {
        load()
        return nativeGetVersion()
    }
}
