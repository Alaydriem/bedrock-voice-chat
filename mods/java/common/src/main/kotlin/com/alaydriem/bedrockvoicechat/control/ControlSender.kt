package com.alaydriem.bedrockvoicechat.control

import com.alaydriem.bedrockvoicechat.network.HttpRequestHandler
import com.alaydriem.bedrockvoicechat.server.BvcServerManager
import org.slf4j.LoggerFactory

/**
 * Routes control actions to either FFI (embedded server) or HTTP `/api/control`
 * (external server), mirroring [com.alaydriem.bedrockvoicechat.audio.AudioEventSender].
 */
class ControlSender(
    private val httpHandler: HttpRequestHandler?,
    private val embeddedServer: BvcServerManager?
) {
    companion object {
        private val logger = LoggerFactory.getLogger("BVC Control")
    }

    /**
     * Routes the action; [onResult] (optional) receives the outcome — synchronously
     * for the FFI path, from the HTTP client's executor for the external path.
     */
    fun send(action: ControlAction, actorId: String, onResult: ((ControlSendResult) -> Unit)? = null) {
        val json = action.toClientActionJson(actorId)
        when {
            embeddedServer != null && embeddedServer.isRunning -> {
                val result = embeddedServer.clientAction(json)
                if (!result.ok) {
                    logger.warn("Failed to send control action via FFI")
                }
                onResult?.invoke(result)
            }
            httpHandler != null -> httpHandler.controlAsync(json) { ok, groupCode ->
                onResult?.invoke(ControlSendResult(ok, groupCode))
            }
            else -> {
                logger.warn("No control sender available (neither embedded nor HTTP configured)")
                onResult?.invoke(ControlSendResult(false))
            }
        }
    }
}
