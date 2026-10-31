package com.alaydriem.bedrockvoicechat.integration

/**
 * The Java name Floodgate gives an unlinked Bedrock player, as `HandshakeDataImpl` builds it:
 * the prefix plus as much of the gamertag as fits in 16 characters, then spaces replaced
 * when `replace-spaces` is on.
 *
 * Test data only. Production code never rebuilds this name; it asks Floodgate for the
 * gamertag instead.
 */
object FloodgateJavaName {
    fun of(gamertag: String, prefix: String, replaceSpaces: Boolean): String {
        val kept = minOf(gamertag.length, 16 - prefix.length)
        val name = prefix + gamertag.substring(0, kept)
        return if (replaceSpaces) name.replace(" ", "_") else name
    }
}
