package com.alaydriem.bedrockvoicechat.powernukkitx.audio

import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class HopperJukeboxRuleTest {

    private val rule = HopperJukeboxRule()

    @Test
    fun a_bvc_disc_feeds_an_empty_idle_jukebox() {
        assertTrue(
            rule.shouldInsert(
                jukeboxHasDisc = false,
                playbackActive = false,
                itemIsBvcDisc = true,
                hopperFacesJukebox = true
            )
        )
        // A hopper that points elsewhere feeds whatever it points at, not this jukebox.
        assertFalse(
            rule.shouldInsert(
                jukeboxHasDisc = false,
                playbackActive = false,
                itemIsBvcDisc = true,
                hopperFacesJukebox = false
            )
        )
    }

    @Test
    fun nothing_feeds_a_jukebox_that_holds_a_disc() {
        assertFalse(
            rule.shouldInsert(
                jukeboxHasDisc = true,
                playbackActive = false,
                itemIsBvcDisc = true,
                hopperFacesJukebox = true
            )
        )
        // The disc can already be out while the stop has not reached the server yet.
        assertFalse(
            rule.shouldInsert(
                jukeboxHasDisc = false,
                playbackActive = true,
                itemIsBvcDisc = true,
                hopperFacesJukebox = true
            )
        )
    }

    @Test
    fun a_non_bvc_item_is_not_fed() {
        assertFalse(
            rule.shouldInsert(
                jukeboxHasDisc = false,
                playbackActive = false,
                itemIsBvcDisc = false,
                hopperFacesJukebox = true
            )
        )
    }

    @Test
    fun a_finished_disc_is_pulled_by_the_hopper_below() {
        assertTrue(
            rule.shouldExtract(
                jukeboxHasDisc = true,
                discIsBvc = true,
                playbackActive = false,
                hopperBelowJukebox = true,
                hopperHasRoom = true
            )
        )
        // Only the hopper directly below pulls from the block above it.
        assertFalse(
            rule.shouldExtract(
                jukeboxHasDisc = true,
                discIsBvc = true,
                playbackActive = false,
                hopperBelowJukebox = false,
                hopperHasRoom = true
            )
        )
        assertFalse(
            rule.shouldExtract(
                jukeboxHasDisc = false,
                discIsBvc = false,
                playbackActive = false,
                hopperBelowJukebox = true,
                hopperHasRoom = true
            )
        )
    }

    @Test
    fun a_playing_disc_is_never_pulled() {
        assertFalse(
            rule.shouldExtract(
                jukeboxHasDisc = true,
                discIsBvc = true,
                playbackActive = true,
                hopperBelowJukebox = true,
                hopperHasRoom = true
            )
        )
    }

    @Test
    fun a_full_hopper_does_not_pull() {
        assertFalse(
            rule.shouldExtract(
                jukeboxHasDisc = true,
                discIsBvc = true,
                playbackActive = false,
                hopperBelowJukebox = true,
                hopperHasRoom = false
            )
        )
    }

    // PowerNukkitX plays a vanilla disc itself; pulling it would cut its song.
    @Test
    fun a_vanilla_disc_is_never_pulled() {
        assertFalse(
            rule.shouldExtract(
                jukeboxHasDisc = true,
                discIsBvc = false,
                playbackActive = false,
                hopperBelowJukebox = true,
                hopperHasRoom = true
            )
        )
    }
}
