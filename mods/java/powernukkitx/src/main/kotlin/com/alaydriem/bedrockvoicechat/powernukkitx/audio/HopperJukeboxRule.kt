package com.alaydriem.bedrockvoicechat.powernukkitx.audio

/**
 * When a hopper moves a disc into or out of a jukebox.
 *
 * PowerNukkitX hoppers do neither: the jukebox is not an inventory holder. The plugin
 * moves BVC discs only. A vanilla disc stays with PowerNukkitX's own jukebox handling.
 */
class HopperJukeboxRule {

    /**
     * A hopper feeds the block its output faces, and only a jukebox that is empty and not
     * still ending a BVC playback.
     */
    fun shouldInsert(
        jukeboxHasDisc: Boolean,
        playbackActive: Boolean,
        itemIsBvcDisc: Boolean,
        hopperFacesJukebox: Boolean
    ): Boolean = itemIsBvcDisc && hopperFacesJukebox && !jukeboxHasDisc && !playbackActive

    /**
     * A hopper directly below pulls a finished BVC disc. A playing disc stays until its
     * track ends.
     */
    fun shouldExtract(
        jukeboxHasDisc: Boolean,
        discIsBvc: Boolean,
        playbackActive: Boolean,
        hopperBelowJukebox: Boolean,
        hopperHasRoom: Boolean
    ): Boolean = jukeboxHasDisc && discIsBvc && !playbackActive && hopperBelowJukebox && hopperHasRoom
}
