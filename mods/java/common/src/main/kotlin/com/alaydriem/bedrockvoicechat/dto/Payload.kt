package com.alaydriem.bedrockvoicechat.dto

/**
 * Payload sent to the BVC server with player position data.
 */
data class Payload(
    val game: String,
    val players: List<PlayerData>
) {
    constructor(gameType: GameType, players: List<PlayerData>) : this(
        game = gameType.value,
        players = players
    )
}
