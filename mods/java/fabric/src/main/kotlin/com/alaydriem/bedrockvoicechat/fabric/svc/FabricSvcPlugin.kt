package com.alaydriem.bedrockvoicechat.fabric.svc

import com.alaydriem.bedrockvoicechat.svc.SvcBridge
import de.maxhenkel.voicechat.api.VoicechatApi
import de.maxhenkel.voicechat.api.VoicechatPlugin
import de.maxhenkel.voicechat.api.events.EventRegistration
import org.slf4j.LoggerFactory

/**
 * The `voicechat` entrypoint Fabric instantiates.
 *
 * Fabric constructs entrypoints with no arguments, and the bridge needs the mod's
 * relay world, peering and platform lookups — none of which exist until the mod
 * initialises. So the entrypoint is a shell that delegates to whatever the mod
 * built, rather than building anything itself.
 *
 * Mod initialisation runs before a world loads and SVC's server starts, so the
 * delegate is set by the time this is asked to register anything. The null branch
 * is a real state rather than a guard against the impossible: a mod whose config
 * was invalid returns early and builds no bridge.
 */
class FabricSvcPlugin : VoicechatPlugin {

    override fun getPluginId(): String = SvcBridge.PLUGIN_ID

    override fun initialize(api: VoicechatApi) {
        delegate?.initialize(api)
    }

    override fun registerEvents(registration: EventRegistration) {
        val bridge = delegate
        if (bridge == null) {
            logger.info("Simple Voice Chat is present but Bedrock Voice Chat built no bridge")
            return
        }
        bridge.registerEvents(registration)
    }

    companion object {
        @Volatile
        var delegate: SvcBridge? = null

        private val logger = LoggerFactory.getLogger("BVC SVC")
    }
}
