package com.alaydriem.bedrockvoicechat.svc.link

/**
 * Which side owns a player's group membership. [NONE] is a player with neither voice client, whom
 * the bridge never moves.
 */
enum class PlayerSide { BVC, SVC, NONE }
