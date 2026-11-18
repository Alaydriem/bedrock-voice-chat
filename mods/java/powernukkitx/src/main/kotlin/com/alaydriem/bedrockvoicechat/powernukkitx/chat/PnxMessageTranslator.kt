package com.alaydriem.bedrockvoicechat.powernukkitx.chat

import org.powernukkitx.lang.TextContainer
import org.powernukkitx.lang.TranslationContainer
import org.powernukkitx.utils.TextFormat

/**
 * Renders a server message as the plain text a player reads.
 *
 * PowerNukkitX sends death, join and quit messages as translation keys that each Bedrock
 * client resolves in its own language. The app has no such table, so a key is resolved
 * here through [translate], the server's language. Plain text is not a key and is never
 * looked up, so a `%` in it stays as typed. Colour codes are dropped from both.
 */
class PnxMessageTranslator(private val translate: (key: String, params: Array<String>) -> String) {

    fun render(message: TextContainer): String {
        val text = if (message is TranslationContainer) {
            translate(message.text, message.parameters)
        } else {
            message.text
        }
        return TextFormat.clean(text)
    }
}
