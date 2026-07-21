package com.alaydriem.bedrockvoicechat.control

/**
 * Outcome of a routed control action. [groupCode] carries the new group's share
 * code after a successful CreateGroup and is null for every other action.
 */
data class ControlSendResult(val ok: Boolean, val groupCode: String? = null)
