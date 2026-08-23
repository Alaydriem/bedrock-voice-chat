package com.alaydriem.bedrockvoicechat.dto

import com.google.gson.annotations.SerializedName

/**
 * Supported game platforms for voice chat integration.
 */
enum class GameType(val value: String) {
    @SerializedName("minecraft")
    MINECRAFT("minecraft");

    /**
     * The key the BVC server indexes a player by: `game:gamertag`.
     *
     * Distinct from the bare gamertag, which is what the position feed carries as a
     * player's name and what a human reads. Everything that looks a player up on the
     * server — the connection registry, channel membership, the control routes — is
     * keyed on this form, and a bare gamertag matches none of them. It does not fail:
     * it answers no, for every player, forever.
     *
     * Mirrors `Game::membership_key` in the Rust common crate, which is the source of
     * truth for the form.
     */
    fun membershipKey(gamertag: String): String = "$value:$gamertag"
}
