package com.alaydriem.bedrockvoicechat.svc

import org.slf4j.LoggerFactory
import java.util.concurrent.atomic.AtomicReference

/**
 * Whether Simple Voice Chat is present on this server.
 *
 * Resolved once and cached tri-state, matching
 * [com.alaydriem.bedrockvoicechat.integration.FloodgateIntegration]: mod load order
 * can put BVC initialization before SVC has registered anything.
 *
 * Only class presence is checked here, and nothing in this file names an SVC type,
 * so it is safe to load on a server that has none. That is what lets the bridge
 * classes — which do name SVC types — stay unloaded until this says yes.
 */
class SvcAvailability(
    private val apiClass: String = API_CLASS
) {
    private enum class State { UNRESOLVED, AVAILABLE, ABSENT }

    private val state = AtomicReference(State.UNRESOLVED)

    val isAvailable: Boolean
        get() = when (state.get()) {
            State.AVAILABLE -> true
            State.ABSENT -> false
            else -> resolve()
        }

    // `initialize = false`: presence is the whole question, and running the class
    // initializer would be a side effect this has no reason to cause.
    private fun resolve(): Boolean = try {
        Class.forName(apiClass, false, javaClass.classLoader)
        state.set(State.AVAILABLE)
        logger.info("Simple Voice Chat detected; bridging voice between SVC and BVC")
        true
    } catch (e: ClassNotFoundException) {
        state.set(State.ABSENT)
        false
    } catch (e: LinkageError) {
        // A present but unloadable API is a broken install, not an absent one, and
        // saying so is the difference between an operator checking their SVC
        // version and concluding BVC ignored it.
        state.set(State.ABSENT)
        logger.warn("Simple Voice Chat is present but its API could not be loaded: {}", e.toString())
        false
    }

    companion object {
        private const val API_CLASS: String = "de.maxhenkel.voicechat.api.VoicechatPlugin"

        private val logger = LoggerFactory.getLogger("BVC SVC")
    }
}
