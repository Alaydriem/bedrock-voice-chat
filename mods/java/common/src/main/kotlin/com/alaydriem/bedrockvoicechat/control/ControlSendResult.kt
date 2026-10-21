package com.alaydriem.bedrockvoicechat.control

/**
 * Outcome of a routed control action. [groupCode] carries the new group's share
 * code after a successful CreateGroup and is null for every other action. [outcome]
 * is set only for a join of a protected group.
 */
data class ControlSendResult(
    val ok: Boolean,
    val groupCode: String? = null,
    val outcome: ControlOutcome? = null,
)
