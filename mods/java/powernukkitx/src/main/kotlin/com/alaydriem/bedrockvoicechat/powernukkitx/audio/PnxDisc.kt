package com.alaydriem.bedrockvoicechat.powernukkitx.audio

import org.powernukkitx.item.Item
import org.powernukkitx.item.ItemID

/**
 * The BVC audio disc: a `music_disc_5` that names the audio it plays.
 *
 * The tags sit on the item's own NBT under the names the Paper and Fabric discs use,
 * `is_bvc_disc` and `audio_id`.
 */
object PnxDisc {
    private const val IS_BVC_DISC_TAG = "is_bvc_disc"
    private const val AUDIO_ID_TAG = "audio_id"

    fun create(audioId: String): Item {
        val disc = Item.get(ItemID.MUSIC_DISC_5)
        disc.setCustomName("BVC: $audioId")
        // The item serializes its NBT to bytes only in setNbt, so a tag changed in place
        // is lost unless it is written back.
        val tag = disc.getOrCreateNbt()
            .putBoolean(IS_BVC_DISC_TAG, true)
            .putString(AUDIO_ID_TAG, audioId)
        disc.setNbt(tag)
        return disc
    }

    fun isBvcDisc(item: Item): Boolean =
        item.getId() == ItemID.MUSIC_DISC_5 && item.getNbt()?.getBoolean(IS_BVC_DISC_TAG) == true

    fun audioId(item: Item): String? {
        if (!isBvcDisc(item)) {
            return null
        }
        val tag = item.getNbt() ?: return null
        if (!tag.containsString(AUDIO_ID_TAG)) {
            return null
        }
        return tag.getString(AUDIO_ID_TAG).ifEmpty { null }
    }
}
