package com.alaydriem.bedrockvoicechat.admin

import com.alaydriem.bedrockvoicechat.dto.GameType
import org.slf4j.LoggerFactory

/**
 * Runs `/bvc admin` and words the reply, so Paper and Fabric differ only in how they
 * register the command and print the result.
 *
 * [target] is read per call: it is null in external mode and after the server stops.
 */
class AdminConsole(
    private val game: GameType,
    private val target: () -> AdminTarget?,
) {
    private val logger = LoggerFactory.getLogger("BVC Admin")

    fun run(action: AdminAction, gamertag: String): String {
        val server = target()
        val result = try {
            server?.admin(AdminRequest(gamertag, game, action)) ?: AdminResult.NOT_EMBEDDED
        } catch (e: UnsatisfiedLinkError) {
            // JNA resolves a symbol on first call, so a native library older than the mod
            // fails here rather than at load.
            logger.error("BVC native library has no bvc_admin export: {}", e.message)
            AdminResult.NATIVE_OUTDATED
        }
        val error = if (result == AdminResult.FAILED) server?.lastError() else null
        return reply(action, gamertag, result, error)
    }

    private fun reply(
        action: AdminAction,
        gamertag: String,
        result: AdminResult,
        error: String?,
    ): String = when (result) {
        AdminResult.APPLIED -> when (action) {
            AdminAction.GRANT -> "Granted BVC admin to $gamertag."
            AdminAction.REVOKE -> "Revoked BVC admin from $gamertag; config defaults now apply."
            AdminAction.DENY -> "Denied BVC admin to $gamertag."
        }
        AdminResult.NOTHING_TO_REVOKE -> "$gamertag has no BVC admin override to revoke."
        AdminResult.PLAYER_NOT_FOUND -> "$gamertag is not a BVC player; nothing was changed."
        AdminResult.FAILED -> "BVC admin change failed: ${error ?: "unknown error"}."
        AdminResult.NOT_EMBEDDED ->
            "This server uses an external BVC server. Run \"bvc-server admin bootstrap -p $gamertag\" on that host."
        AdminResult.NATIVE_OUTDATED ->
            "The BVC native library predates /bvc admin. Update the plugin and restart the server."
    }
}
