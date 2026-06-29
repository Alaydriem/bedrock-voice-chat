package com.alaydriem.bedrockvoicechat.fabric.mixin;

import com.alaydriem.bedrockvoicechat.fabric.audio.JukeboxListener;
import net.minecraft.world.Container;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;
import net.minecraft.world.level.block.entity.JukeboxBlockEntity;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfoReturnable;

@Mixin(JukeboxBlockEntity.class)
public abstract class JukeboxBlockEntityMixin {

    // BVC discs have JUKEBOX_PLAYABLE stripped, which vanilla canPlaceItem rejects; allow them
    // into an empty jukebox so hoppers (which gate on canPlaceItem) can insert them.
    @Inject(method = "canPlaceItem", at = @At("HEAD"), cancellable = true)
    private void bvc$allowBvcDisc(int slot, ItemStack stack, CallbackInfoReturnable<Boolean> cir) {
        if (slot != 0) return;
        if (!JukeboxListener.Companion.isBvcDisc(stack)) return;
        JukeboxBlockEntity self = (JukeboxBlockEntity) (Object) this;
        cir.setReturnValue(self.getItem(slot).isEmpty());
    }

    // While BVC audio is playing for this jukebox, lock the disc in place so hoppers cannot pull it;
    // once playback ends and the disc is ejected, the dropped item is picked up normally.
    @Inject(method = "canTakeItem", at = @At("HEAD"), cancellable = true)
    private void bvc$lockWhilePlaying(Container into, int slot, ItemStack stack, CallbackInfoReturnable<Boolean> cir) {
        JukeboxBlockEntity self = (JukeboxBlockEntity) (Object) this;
        if (JukeboxListener.Companion.isPlaybackActive(self)) {
            cir.setReturnValue(false);
        }
    }

    @Inject(method = "setTheItem", at = @At("HEAD"))
    private void bvc$onSetItem(ItemStack stack, CallbackInfo ci) {
        if (stack == null || stack.isEmpty()) return;
        if (stack.getItem() != Items.MUSIC_DISC_5) return;
        if (!JukeboxListener.Companion.isBvcDisc(stack)) return;

        JukeboxBlockEntity self = (JukeboxBlockEntity) (Object) this;
        JukeboxListener.Companion.onHopperInsert(self, stack);
    }
}
