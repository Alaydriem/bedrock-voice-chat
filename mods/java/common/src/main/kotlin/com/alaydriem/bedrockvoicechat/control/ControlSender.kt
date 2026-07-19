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

    fun send(action: ControlAction, actorId: String) {
        val json = action.toClientActionJson(actorId)
        when {
            embeddedServer != null && embeddedServer.isRunning -> {
                if (!embeddedServer.clientAction(json)) {
                    logger.warn("Failed to send control action via FFI")
                }
            }
            httpHandler != null -> httpHandler.controlAsync(json)
            else -> logger.warn("No control sender available (neither embedded nor HTTP configured)")
        }
    }
}
