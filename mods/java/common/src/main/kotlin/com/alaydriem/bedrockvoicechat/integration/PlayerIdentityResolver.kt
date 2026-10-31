package com.alaydriem.bedrockvoicechat.integration

import org.slf4j.LoggerFactory
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap

/**
 * Resolves a player's BVC identity from their UUID and in-game name.
 *
 * Shared by every platform so the rule exists once. Floodgate's gamertag is used as given;
 * the Java name Floodgate builds from it (prefix, truncation, replaced spaces) is never
 * reconstructed, because that rule is Floodgate's and can change.
 */
class PlayerIdentityResolver(
    // The Xbox gamertag Floodgate holds for a player, null for anyone it does not know.
    private val xboxGamertagOf: (UUID) -> String?
) {
    private val log = LoggerFactory.getLogger("BedrockVoiceChat.Identity")
    private val loggedIdentities: MutableSet<String> = ConcurrentHashMap.newKeySet()
    private val warnedMissingApi: MutableSet<UUID> = ConcurrentHashMap.newKeySet()

    fun resolve(uuid: UUID, javaName: String): PlayerIdentity {
        val gamertag = xboxGamertagOf(uuid)
        // Floodgate stores an unlinked Bedrock player under UUID(0, xuid); a linked one
        // carries their Java account's UUID.
        val floodgateUuid = uuid.mostSignificantBits == 0L

        val identity = if (gamertag == null) {
            if (floodgateUuid && warnedMissingApi.add(uuid)) {
                log.warn(
                    "A Floodgate player was seen but the Floodgate API is not available on this server; " +
                        "install Floodgate on the backend, not only the proxy: uuid={}, javaName='{}'",
                    uuid, javaName
                )
            }
            PlayerIdentity(javaName, null, false)
        } else {
            PlayerIdentity(gamertag, javaName.takeIf { it != gamertag }, !floodgateUuid)
        }

        if (loggedIdentities.add("$uuid|$javaName|$gamertag")) {
            log.info(
                "Identity resolution: uuid={}, javaName='{}', gamertag='{}' -> name='{}', java='{}', linked={}, reported=('{}', '{}')",
                uuid, javaName, gamertag, identity.name, identity.javaName, identity.linked,
                identity.reportedName, identity.reportedAlternative
            )
        }
        return identity
    }

    /**
     * The candidate a name refers to, or null when nobody answers to it.
     *
     * An exact gamertag wins over an exact Java name, which wins over either matched
     * without case. Within a tier the first candidate wins. A linked player's Java name
     * may be another player's gamertag; the gamertag's owner must be the one found.
     */
    fun <T> bestMatch(name: String, candidates: List<Pair<T, PlayerIdentity>>): T? {
        val tiers: List<(PlayerIdentity) -> Boolean> = listOf(
            { it.name == name },
            { it.javaName == name },
            { it.name.equals(name, ignoreCase = true) },
            { it.javaName?.equals(name, ignoreCase = true) == true }
        )
        for (matches in tiers) {
            candidates.firstOrNull { matches(it.second) }?.let { return it.first }
        }
        return null
    }
}
