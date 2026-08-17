package com.alaydriem.bedrockvoicechat.native

import com.alaydriem.bedrockvoicechat.control.ControlSendResult
import com.sun.jna.Library
import com.sun.jna.Native
import com.sun.jna.Pointer
import com.sun.jna.ptr.PointerByReference
import org.slf4j.LoggerFactory

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
        fun bvc_audio_play(handle: Pointer, playJson: String): Pointer?
        fun bvc_audio_stop(handle: Pointer, eventId: String): Int
        fun bvc_client_action(handle: Pointer, actionJson: String, groupCodeOut: PointerByReference?): Int
        fun bvc_chat_register(handle: Pointer, helloJson: String): Int
        fun bvc_chat_report(handle: Pointer, chatJson: String): Int
        fun bvc_host_capability(handle: Pointer, reportJson: String): Int
        fun bvc_relay_peerlink(handle: Pointer): Pointer?
        fun bvc_has_live_client(handle: Pointer, identity: String): Int
        fun bvc_chat_drain(handle: Pointer): Pointer?
        fun bvc_config_effective(handle: Pointer): Pointer?
        fun bvc_chat_unregister(handle: Pointer): Int
        fun bvc_free_string(ptr: Pointer)
        fun bvc_get_last_error(): String?
        fun bvc_version(): String
        fun bvc_protocol_version(): String
    }

    const val LIBRARY_NAME: String = "bvc_server_lib"

    private var provider: NativeLibraryProvider? = null

    /**
     * Supplies the resolver. Called during platform startup, before any FFI call.
     */
    @Synchronized
    fun configure(provider: NativeLibraryProvider?) {
        this.provider = provider
    }

    /**
     * Loads the native library from the path the provider resolved.
     *
     * There is no system-path fallback. Falling back would load any library of
     * that name already present on the host, which is exactly the unverified load
     * the pinned digest exists to prevent.
     */
    @Synchronized
    fun load() {
        if (library != null) return

        val resolver = provider
            ?: throw IllegalStateException(
                "No native library provider configured; call BvcNative.configure before any FFI call"
            )

        val libFile = resolver.resolve(LIBRARY_NAME)
        logger.info("Loading native library: {}", libFile.absolutePath)

        try {
            library = Native.load(libFile.absolutePath, BvcLibrary::class.java)
            logger.info("Loaded native library successfully")
        } catch (e: Exception) {
            logger.error("Failed to load native library: {}", e.message, e)
            throw RuntimeException("Failed to load BVC native library", e)
        }

        val initResult = library!!.bvc_init()
        if (initResult != 0) {
            logger.warn("Crypto provider init returned: {} (may already be initialized)", initResult)
        }
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
     * Start audio playback via FFI.
     *
     * @param handle Server handle from createServer
     * @param playJson JSON string matching AudioPlayRequest structure
     * @return JSON string with AudioEventResponse on success, null on failure
     */
    fun audioPlay(handle: Pointer, playJson: String): String? {
        val ptr = getLib().bvc_audio_play(handle, playJson) ?: run {
            logger.warn("Failed to start audio playback: {}", getLastError())
            return null
        }
        try {
            return ptr.getString(0)
        } finally {
            getLib().bvc_free_string(ptr)
        }
    }

    /**
     * Stop audio playback via FFI.
     *
     * @param handle Server handle from createServer
     * @param eventId Event ID to stop
     * @return 0 on success, -1 on error
     */
    fun audioStop(handle: Pointer, eventId: String): Int {
        val result = getLib().bvc_audio_stop(handle, eventId)
        if (result != 0) {
            logger.warn("Failed to stop audio playback: {}", getLastError())
        }
        return result
    }

    /**
     * Submit an in-game control action via FFI.
     *
     * @param handle Server handle from createServer
     * @param actionJson JSON string matching the common ClientAction structure
     * @return the outcome; groupCode carries the share code after a successful CreateGroup
     */
    fun clientAction(handle: Pointer, actionJson: String): ControlSendResult {
        val codeOut = PointerByReference()
        val result = getLib().bvc_client_action(handle, actionJson, codeOut)
        if (result != 0) {
            logger.warn("Failed to send control action: {}", getLastError())
            return ControlSendResult(false)
        }
        val ptr = codeOut.value ?: return ControlSendResult(true)
        try {
            return ControlSendResult(true, ptr.getString(0))
        } finally {
            getLib().bvc_free_string(ptr)
        }
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

    /**
     * Get the protocol version string.
     */
    fun getProtocolVersion(): String {
        return getLib().bvc_protocol_version()
    }

    /** Registers the embedded mod as this world's chat channel. */
    fun chatRegister(handle: Pointer, helloJson: String): Boolean =
        getLib().bvc_chat_register(handle, helloJson) == 0

    /** Reports a line a player typed in game. */
    fun chatReport(handle: Pointer, chatJson: String): Boolean =
        getLib().bvc_chat_report(handle, chatJson) == 0

    /** Reports whether this host could fetch and write a native library. */
    fun hostCapability(handle: Pointer, reportJson: String): Boolean =
        getLib().bvc_host_capability(handle, reportJson) == 0

    /**
     * This server's own peerlink, for a bridge running beside it.
     *
     * Null when the server declares no peers, in which case it binds no peer
     * endpoint and there is nothing for a bridge to dial.
     */
    fun relayPeerlink(handle: Pointer): String? {
        val ptr = getLib().bvc_relay_peerlink(handle) ?: return null
        return try {
            ptr.getString(0)
        } finally {
            getLib().bvc_free_string(ptr)
        }
    }

    /** Whether a player holds a live voice connection to this server. */
    fun hasLiveClient(handle: Pointer, identity: String): Boolean =
        getLib().bvc_has_live_client(handle, identity) == 1

    /**
     * Takes every `say` frame waiting to be broadcast, as a JSON array.
     */
    fun chatDrain(handle: Pointer): String? {
        val ptr = getLib().bvc_chat_drain(handle) ?: return null
        return try {
            ptr.getString(0)
        } finally {
            getLib().bvc_free_string(ptr)
        }
    }

    /** Releases every chat room this mod registered. */
    fun chatUnregister(handle: Pointer): Boolean =
        getLib().bvc_chat_unregister(handle) == 0

    /** The configuration the server resolved, as JSON, after defaults and BVC_* overrides. */
    fun configEffective(handle: Pointer): String? {
        val ptr = getLib().bvc_config_effective(handle) ?: return null
        return try {
            ptr.getString(0)
        } finally {
            getLib().bvc_free_string(ptr)
        }
    }
}
