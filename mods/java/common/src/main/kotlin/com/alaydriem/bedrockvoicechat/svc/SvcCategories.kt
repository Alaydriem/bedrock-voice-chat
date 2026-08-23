package com.alaydriem.bedrockvoicechat.svc

import de.maxhenkel.voicechat.api.VoicechatServerApi

/**
 * The volume category jukebox playback arrives under.
 *
 * Separate from speech so an SVC player can turn music down without turning people
 * down. Registered once, when the server API becomes available.
 */
object SvcCategories {

    const val JUKEBOX_ID: String = "bvc_jukebox"

    fun register(api: VoicechatServerApi) {
        api.registerVolumeCategory(
            api.volumeCategoryBuilder()
                .setId(JUKEBOX_ID)
                .setName("Jukebox")
                .setDescription("Music and audio played through Bedrock Voice Chat")
                .build()
        )
    }
}
