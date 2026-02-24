package com.alaydriem.bedrockvoicechat.fabric.audio

import net.minecraft.component.type.TooltipDisplayComponent
import net.minecraft.item.Item
import net.minecraft.item.ItemStack
import net.minecraft.item.tooltip.TooltipType
import net.minecraft.text.Text
import net.minecraft.util.Formatting
import java.util.function.Consumer

/**
 * Custom audio disc item for Fabric.
 * Carries a BVC audio file ID in its data components.
 */
class AudioDiscItem(settings: Settings) : Item(settings) {

    @Suppress("OVERRIDE_DEPRECATION")
    override fun appendTooltip(
        stack: ItemStack,
        context: TooltipContext,
        displayComponent: TooltipDisplayComponent,
        tooltip: Consumer<Text>,
        type: TooltipType
    ) {
        val audioId = stack.get(FabricAudioRegistry.BVC_AUDIO_ID)
        if (audioId != null) {
            tooltip.accept(Text.literal("Audio: $audioId").formatted(Formatting.GRAY))
        }
    }
}
