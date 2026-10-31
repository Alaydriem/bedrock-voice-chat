package com.alaydriem.bedrockvoicechat.control

/**
 * What happened to a non-protected BVC channel. Names match common's `GroupLinkEventKind`.
 * `Protected` is the last event a channel produces.
 */
enum class GroupLinkEventKind { Create, Join, Leave, Delete, Protected }
