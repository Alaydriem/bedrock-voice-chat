package com.alaydriem.bedrockvoicechat.fabric.svc

import com.alaydriem.bedrockvoicechat.svc.SvcBridge
import de.maxhenkel.voicechat.api.VoicechatApi
import de.maxhenkel.voicechat.api.VoicechatPlugin
import de.maxhenkel.voicechat.api.events.EventRegistration
import de.maxhenkel.voicechat.api.events.MicrophonePacketEvent
import de.maxhenkel.voicechat.api.events.VoicechatServerStartedEvent
import org.slf4j.LoggerFactory

/**
 * The `voicechat` entrypoint Fabric instantiates.
 *
 * Simple Voice Chat asks for events during mod initialisation, before any server
 * exists. The bridge cannot be built that early: its channel factory and speaker
 * lookup both need a running `MinecraftServer`. So this registers handlers of its
 * own immediately and resolves the bridge when an event fires.
 *
 * Neither order is guaranteed after that. Simple Voice Chat starts its server from
 * its own hook and Fabric fires `SERVER_STARTED` from another, so the event that
 * opens the session can arrive before or after the bridge is built. Whichever comes
 * second completes the handoff, so neither ordering drops it.
 */
class FabricSvcPlugin : VoicechatPlugin {

    override fun getPluginId(): String = SvcBridge.PLUGIN_ID

    override fun initialize(api: VoicechatApi) {
        // Nothing to do. The bridge does its setup from the server-started event,
        // which carries the server API this one does not.
    }

    override fun registerEvents(registration: EventRegistration) {
        registration.registerEvent(VoicechatServerStartedEvent::class.java) { event ->
            onServerStarted(event)
        }

        registration.registerEvent(MicrophonePacketEvent::class.java) { event ->
            // Dropped until the bridge exists. Those frames are the first moments of
            // a server start, when nobody is in the world to be heard.
            bridge?.onMicrophonePacket(event)
        }
    }

    companion object {
        private val logger = LoggerFactory.getLogger("BVC SVC")

        @Volatile
        private var bridge: SvcBridge? = null

        @Volatile
        private var pending: VoicechatServerStartedEvent? = null

        /** Called by the mod once a server exists and the bridge can be built. */
        @Synchronized
        fun attach(built: SvcBridge) {
            bridge = built

            val waiting = pending ?: return
            pending = null
            logger.info("Simple Voice Chat started before the bridge was built; connecting now")
            built.onServerStarted(waiting)
        }

        @Synchronized
        fun detach() {
            bridge = null
            pending = null
        }

        @Synchronized
        private fun onServerStarted(event: VoicechatServerStartedEvent) {
            val built = bridge
            if (built == null) {
                pending = event
                return
            }
            built.onServerStarted(event)
        }
    }
}
