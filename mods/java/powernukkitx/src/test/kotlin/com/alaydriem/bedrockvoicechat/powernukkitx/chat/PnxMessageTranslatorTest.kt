package com.alaydriem.bedrockvoicechat.powernukkitx.chat

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test
import org.powernukkitx.lang.BaseLang
import org.powernukkitx.lang.TextContainer
import org.powernukkitx.lang.TranslationContainer

class PnxMessageTranslatorTest {

    private val translator = PnxMessageTranslator { key, params -> LANG.tr(key, *params) }

    @Test
    fun a_death_translation_key_is_rendered_as_text() {
        val message = TranslationContainer("death.attack.generic", "Steve")

        assertEquals("Steve died", translator.render(message))
    }

    @Test
    fun a_join_message_is_rendered_without_its_colour_code() {
        val message = TranslationContainer("§e%multiplayer.player.joined", "Steve")

        assertEquals("Steve joined the game", translator.render(message))
    }

    @Test
    fun plain_chat_passes_through_unchanged() {
        val message = TextContainer("<Steve> 100% sure %death.attack.generic is a key")

        assertEquals("<Steve> 100% sure %death.attack.generic is a key", translator.render(message))
    }

    companion object {
        private val LANG = BaseLang("eng")
    }
}
