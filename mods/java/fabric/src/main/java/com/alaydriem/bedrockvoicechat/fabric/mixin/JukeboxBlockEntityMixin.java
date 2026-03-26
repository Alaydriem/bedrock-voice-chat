package com.alaydriem.bedrockvoicechat.fabric.mixin;

import com.alaydriem.bedrockvoicechat.fabric.audio.JukeboxListener;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;
import net.minecraft.world.level.block.entity.JukeboxBlockEntity;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.Shadow;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(JukeboxBlockEntity.class)
public abstract class JukeboxBlockEntityMixin {

    @Shadow
    public abstract ItemStack getTheItem();

    @Inject(method = "notifyItemChangedInJukebox", at = @At("HEAD"))
    private void bvc$onRecordChanged(boolean hasRecord, CallbackInfo ci) {
        if (!hasRecord) return;
        ItemStack stack = this.getTheItem();
        if (stack.isEmpty()) return;
        if (stack.getItem() != Items.MUSIC_DISC_5) return;
        if (!JukeboxListener.Companion.isBvcDisc(stack)) return;

        JukeboxBlockEntity self = (JukeboxBlockEntity) (Object) this;
        JukeboxListener.Companion.onHopperInsert(self, stack);
    }
}
