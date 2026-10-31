package com.alaydriem.bedrockvoicechat.svc

/** What the bridge does with a Java speaker's microphone packet, by the speaker's SVC group. */
sealed class GroupRoute {
    /** No group, or an OPEN group that is not linked: ordinary proximity speech. */
    object Proximity : GroupRoute()

    /** A linked group: sent with the group marker. [membersOnly] speech never reaches proximity. */
    data class Group(val channel: String, val membersOnly: Boolean) : GroupRoute()

    /** A NORMAL or ISOLATED group that is not linked: its speech has nowhere it may go. */
    object Drop : GroupRoute()
}
