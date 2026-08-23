package com.alaydriem.bedrockvoicechat.native

import com.alaydriem.bedrockvoicechat.network.HttpRequestHandler
import com.alaydriem.bedrockvoicechat.server.BvcServerManager
import com.google.gson.Gson
import org.slf4j.LoggerFactory

/**
 * Routes a host capability report to the BVC server the mod is already talking to,
 * over FFI when embedded and HTTP when external.
 *
 * Mirrors [com.alaydriem.bedrockvoicechat.control.ControlSender]. The mod has no
 * telemetry channel of its own, and the hosts that matter most to this measurement
 * are external-mode ones with no embedded server to borrow one from.
 */
class HostCapabilitySender(
    private val httpHandler: HttpRequestHandler?,
    private val embeddedServer: BvcServerManager?
) {
    fun send(report: HostCapabilityReport) {
        val json = Gson().toJson(report)
        when {
            embeddedServer != null && embeddedServer.isRunning -> embeddedServer.hostCapability(json)
            httpHandler != null -> httpHandler.hostCapabilityAsync(json)
            else -> logger.debug("No sender available for the host capability report")
        }
    }

    companion object {
        private val logger = LoggerFactory.getLogger("BVC Native")
    }
}
