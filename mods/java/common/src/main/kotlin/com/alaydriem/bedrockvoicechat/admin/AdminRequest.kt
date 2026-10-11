package com.alaydriem.bedrockvoicechat.admin

import com.alaydriem.bedrockvoicechat.dto.GameType

/** Mirrors common's `AdminPermissionRequest`, the JSON `bvc_admin` takes. */
data class AdminRequest(
    val gamertag: String,
    val game: GameType,
    val action: AdminAction,
)
