package com.alaydriem.bedrockvoicechat.svc

import de.maxhenkel.voicechat.api.VoicechatApi
import de.maxhenkel.voicechat.api.VoicechatPlugin
import de.maxhenkel.voicechat.api.VoicechatServerApi
import de.maxhenkel.voicechat.api.events.EventRegistration
import de.maxhenkel.voicechat.api.events.MicrophonePacketEvent
import de.maxhenkel.voicechat.api.events.VoicechatServerStartedEvent
import org.slf4j.LoggerFactory

/**
 * The bridge, as Simple Voice Chat sees it.
 *
 * A tee rather than a hub: the microphone event is observed and never cancelled, so
 * SVC keeps delivering its own players to each other natively. Only cross-population
 * audio crosses — SVC speakers out to BVC, BVC speakers in to SVC.
 *
 * This class names SVC types, so it must not be loaded until [SvcAvailability] says
 * they are present.
 */
class SvcBridge(
    private val outbound: OutboundTranslator,
    private val onFrame: (uniffi.bvc_relay_sdk.SdkFrame) -> Unit,
    private val onServerApi: (VoicechatServerApi) -> Unit
) : VoicechatPlugin {

    override fun getPluginId(): String = PLUGIN_ID

    override fun initialize(api: VoicechatApi) {
        logger.info("Simple Voice Chat bridge initialising")
    }

    override fun registerEvents(registration: EventRegistration) {
        registration.registerEvent(VoicechatServerStartedEvent::class.java, this::onServerStarted)
        registration.registerEvent(MicrophonePacketEvent::class.java, this::onMicrophonePacket)
    }

    private fun onServerStarted(event: VoicechatServerStartedEvent) {
        onServerApi(event.voicechat)
        logger.info("Simple Voice Chat server API acquired")
    }

    /**
     * Observed, never cancelled.
     *
     * Cancelling would stop SVC delivering to its own players, and the BVC server
     * never returns a peer's own frames to that peer — so there would be nothing to
     * deliver them instead and SVC players would go silent to each other.
     */
    private fun onMicrophonePacket(event: MicrophonePacketEvent) {
        val speaker = event.senderConnection?.player?.uuid ?: return
        val frame = outbound.translate(
            speaker,
            event.packet.opusEncodedData,
            System.currentTimeMillis()
        ) ?: return

        onFrame(frame)
    }

    companion object {
        const val PLUGIN_ID: String = "bedrock-voice-chat"

        private val logger = LoggerFactory.getLogger("BVC SVC")
    }
}
