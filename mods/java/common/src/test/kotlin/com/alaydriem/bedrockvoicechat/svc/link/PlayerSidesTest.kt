package com.alaydriem.bedrockvoicechat.svc.link

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test
import java.util.UUID

class PlayerSidesTest {
    private val player = UUID.randomUUID()

    // A Java account using the BVC client without the SVC mod speaks through BVC, so BVC owns
    // their membership. The client wins even when SVC also reports an install.
    @Test
    fun `live_bvc_client_wins_over_svc_install`() {
        val sides = PlayerSides(hasLiveBvcClient = { true }, hasSvcInstalled = { true })

        assertEquals(PlayerSide.BVC, sides.of(player))
    }

    @Test
    fun `svc_installed_without_bvc_is_svc`() {
        val sides = PlayerSides(hasLiveBvcClient = { false }, hasSvcInstalled = { true })

        assertEquals(PlayerSide.SVC, sides.of(player))
    }

    // A Geyser player with no BVC client is not a voice participant; the bridge does not move them.
    @Test
    fun `neither_is_none`() {
        val sides = PlayerSides(hasLiveBvcClient = { false }, hasSvcInstalled = { false })

        assertEquals(PlayerSide.NONE, sides.of(player))
    }
}
