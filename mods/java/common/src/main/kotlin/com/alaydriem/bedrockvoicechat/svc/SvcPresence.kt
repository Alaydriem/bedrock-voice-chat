package com.alaydriem.bedrockvoicechat.svc

import java.util.UUID

/**
 * Shows players who are on voice through BVC as connected in Simple Voice Chat.
 *
 * SVC marks anyone its own server holds no connection for as disconnected, so a
 * player speaking through the BVC app carried that mark over their head for every
 * Java player standing next to them — while both sides could hear each other.
 *
 * `setConnected` is SVC's own answer: it clears the disconnected flag and
 * broadcasts the new state to every client. It declines for a player who has SVC
 * installed, whose state is already their real one, so a player running both is
 * left alone.
 *
 * Reconciled on a sweep rather than set once. SVC resets the flag whenever a
 * player's real state changes or they reconnect, and a BVC client can close at any
 * moment, so the mark has to be both applied and withdrawn continuously. Every call
 * is idempotent — SVC returns early when the state already matches — which is what
 * makes sweeping cheap enough to do on a timer.
 *
 * `setConnected` is a lambda rather than a `VoicechatServerApi`, for the reason
 * [SvcChannelFactory] gives: that interface declares dozens of methods and this
 * needs one of them.
 */
class SvcPresence(
    private val onlinePlayers: () -> List<UUID>,
    private val hasLiveBvcClient: (UUID) -> Boolean,
    private val setConnected: (UUID, Boolean) -> Unit
) {

    /**
     * Must run on the server thread: the online player list is owned by it, and
     * reading it while it is being mutated is the one part of this that is not
     * already safe to call from anywhere.
     */
    fun reconcile() {
        for (player in onlinePlayers()) {
            setConnected(player, hasLiveBvcClient(player))
        }
    }
}
