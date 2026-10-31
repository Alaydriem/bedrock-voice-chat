package com.alaydriem.bedrockvoicechat.dto

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNotEquals
import org.junit.jupiter.api.Test

class DimensionTest {
    @Test
    fun `vanilla identifiers map to the vanilla dimensions`() {
        assertEquals(Dimension.Minecraft.OVERWORLD, Dimension.fromIdentifier("minecraft:overworld"))
        assertEquals(Dimension.Minecraft.NETHER, Dimension.fromIdentifier("minecraft:the_nether"))
        assertEquals(Dimension.Minecraft.THE_END, Dimension.fromIdentifier("minecraft:the_end"))
    }

    // The jukebox once sent only the identifier's path, so a disc in a modded dimension
    // matched no player standing next to it.
    @Test
    fun `a modded identifier is sent whole`() {
        assertEquals(
            "twilightforest:twilight_forest",
            Dimension.fromIdentifier("twilightforest:twilight_forest").toApiString()
        )
    }

    @Test
    fun `two mods with the same path stay apart`() {
        assertNotEquals(
            Dimension.fromIdentifier("modA:foo").toApiString(),
            Dimension.fromIdentifier("modB:foo").toApiString()
        )
    }
}
