package com.alaydriem.bedrockvoicechat.integration

/**
 * Who a player is to BVC, and how the mod reports them to the BVC server.
 *
 * [name] is the BVC name: the Xbox gamertag for any player Floodgate knows, because that
 * is the name their BVC client signs in with. [javaName] is the in-game profile name when
 * it differs. [linked] is true for a Floodgate player known by a Java account's UUID.
 */
data class PlayerIdentity(
    val name: String,
    val javaName: String?,
    val linked: Boolean
) {
    // A linked player is reported as (Java name, gamertag), the shape beta.27 sent, so the
    // server can keep the Java name as an alias for when they join from Java Edition. The
    // exception is a Java name ending with the gamertag: beta.27 reported that player under
    // the gamertag, and an older server only handles them if that name is kept.
    private val reportsJavaName: Boolean
        get() = linked && javaName != null && !javaName.endsWith(name)

    // A linked player carries a Java account UUID, which proves nothing about a gamertag.
    // Reporting the gamertag as its own alternative tells the server Floodgate vouched for it.
    private val reportsOwnName: Boolean
        get() = linked && javaName != null && javaName.endsWith(name)

    val reportedName: String
        get() = if (reportsJavaName) javaName!! else name

    val reportedAlternative: String?
        get() = if (reportsJavaName || reportsOwnName) name else null
}
