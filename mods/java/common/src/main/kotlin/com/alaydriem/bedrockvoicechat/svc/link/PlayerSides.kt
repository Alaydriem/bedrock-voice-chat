package com.alaydriem.bedrockvoicechat.svc.link

import java.util.UUID

/**
 * Decides a player's side at the moment of each decision. A live BVC client wins: that player's
 * voice comes through BVC whatever else is installed.
 */
class PlayerSides(
    private val hasLiveBvcClient: (UUID) -> Boolean,
    private val hasSvcInstalled: (UUID) -> Boolean,
) {
    fun of(player: UUID): PlayerSide = when {
        hasLiveBvcClient(player) -> PlayerSide.BVC
        hasSvcInstalled(player) -> PlayerSide.SVC
        else -> PlayerSide.NONE
    }
}
