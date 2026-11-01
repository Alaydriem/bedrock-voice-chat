package com.alaydriem.bedrockvoicechat.dto

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNotEquals
import org.junit.jupiter.api.Test

class DimensionTest {
    private fun of(identifier: String): Dimension = Dimension.fromString(GameType.MINECRAFT, identifier)!!

    @Test
    fun `vanilla identifiers map to the vanilla dimensions`() {
        assertEquals(Dimension.Minecraft.OVERWORLD, of("minecraft:overworld"))
        assertEquals(Dimension.Minecraft.NETHER, of("minecraft:the_nether"))
        assertEquals(Dimension.Minecraft.THE_END, of("minecraft:the_end"))
    }

    // The jukebox once sent only the identifier's path, so a disc in a modded dimension
    // matched no player standing next to it.
    @Test
    fun `a modded identifier is sent whole`() {
        assertEquals("twilightforest:twilight_forest", of("twilightforest:twilight_forest").toApiString())
    }

    @Test
    fun `two mods with the same path stay apart`() {
        assertNotEquals(of("modA:foo").toApiString(), of("modB:foo").toApiString())
    }
}
