import { system, EquipmentSlot, ItemStack } from '@minecraft/server';
import type {
  Block,
  BlockComponentPlayerInteractEvent,
  BlockComponentPlayerBreakEvent,
  BlockComponentTickEvent,
  CustomComponentParameters,
} from '@minecraft/server';
import { AudioPlayerManager } from './audio-player-manager';

/**
 * Helper: set the bvc:playing block state (controls redstone_producer permutation).
 */
function setBlockPlaying(block: Block, playing: boolean): void {
  try {
    block.setPermutation(
      block.permutation.withState('bvc:playing' as any, playing)
    );
  } catch (e) {
    console.error('[BVC] Failed to set bvc:playing state:', e);
  }
}

/**
 * Helper: try to find a BVC disc in a block's inventory component.
 * Returns the audioId if found and removes the item, or null.
 */
function pullDiscFromBlockInventory(block: Block): string | null {
  try {
    const inv = block.getComponent('minecraft:inventory') as any;
    if (!inv?.container) return null;

    const item = inv.container.getItem(0);
    if (!item) return null;

    const isBvc = item.getDynamicProperty('bvc:is_bvc_disc');
    if (!isBvc) return null;

    const audioId = item.getDynamicProperty('bvc:audio_id') as string | undefined;
    if (!audioId) return null;

    inv.container.setItem(0, undefined);
    return audioId;
  } catch {
    return null;
  }
}

/**
 * Adjacent input positions and the hopper facing_direction required to point at our block.
 * facing_direction: 0=down, 2=north(-z), 3=south(+z), 4=west(-x), 5=east(+x)
 * A hopper at (x+1, y, z) must face west (4) to push toward our block at (x, y, z).
 */
const INPUT_NEIGHBORS: { x: number; y: number; z: number; facing: number }[] = [
  { x: 0, y: 1, z: 0, facing: 0 },   // above  → must face down
  { x: 1, y: 0, z: 0, facing: 4 },   // east   → must face west
  { x: -1, y: 0, z: 0, facing: 5 },  // west   → must face east
  { x: 0, y: 0, z: 1, facing: 2 },   // south  → must face north
  { x: 0, y: 0, z: -1, facing: 3 },  // north  → must face south
];

/**
 * Helper: try to find a BVC disc in an adjacent hopper that is pointing at our block.
 * Returns the audioId if found and removes the item, or null.
 */
function pullDiscFromAdjacentHopper(block: Block): string | null {
  for (const n of INPUT_NEIGHBORS) {
    try {
      const neighbor = block.dimension.getBlock({
        x: block.x + n.x,
        y: block.y + n.y,
        z: block.z + n.z,
      });
      if (!neighbor || !neighbor.typeId.includes('hopper')) continue;

      // Check the hopper is actually pointing at our block
      const facing = neighbor.permutation.getState('facing_direction' as any);
      if (facing !== n.facing) continue;

      const inv = neighbor.getComponent('minecraft:inventory') as any;
      if (!inv?.container) continue;

      for (let slot = 0; slot < inv.container.size; slot++) {
        const item = inv.container.getItem(slot);
        if (!item) continue;

        const isBvc = item.getDynamicProperty('bvc:is_bvc_disc');
        if (!isBvc) continue;

        const audioId = item.getDynamicProperty('bvc:audio_id') as string | undefined;
        if (!audioId) continue;

        inv.container.setItem(slot, undefined);
        return audioId;
      }
    } catch {
      // Skip this neighbor if it can't be read
    }
  }
  return null;
}

/**
 * Registers the bvc:audio_player block custom component.
 *
 * Flow:
 *   1. Player right-clicks with BVC disc → insert + start playback
 *   2. Player right-clicks (disc loaded) → eject + stop playback
 *   3. Playback finishes → auto-eject disc (to block inv / hopper below / world item)
 *   4. onTick detects hopper/inventory insertions → start playback
 *   5. Redstone signal emitted during playback (locks adjacent hoppers)
 */
export function registerAudioComponents(
  audioManager: AudioPlayerManager,
  getWorldUuid: () => string
): void {
  system.beforeEvents.startup.subscribe((event) => {
    event.blockComponentRegistry.registerCustomComponent('bvc:audio_player', {
      onPlayerInteract(
        e: BlockComponentPlayerInteractEvent,
        _params: CustomComponentParameters
      ) {
        const block = e.block;
        const player = e.player;
        if (!player) return;

        const worldUuid = getWorldUuid();
        const locationKey = audioManager.locationKey(
          worldUuid,
          block.x,
          block.y,
          block.z
        );

        if (audioManager.hasDisc(locationKey)) {
          // Eject disc - stop playback and return to player
          const audioId = audioManager.ejectDisc(locationKey);
          audioManager.killMarkers(block.dimension, block.x, block.y, block.z);
          setBlockPlaying(block, false);

          if (audioId) {
            try {
              const disc = new ItemStack('minecraft:music_disc_5', 1);
              disc.nameTag = `BVC: ${audioId}`;
              disc.setDynamicProperty('bvc:audio_id', audioId);
              disc.setDynamicProperty('bvc:is_bvc_disc', true);

              const inventory = player.getComponent('minecraft:inventory');
              if (inventory?.container) {
                inventory.container.addItem(disc);
              }
            } catch (err) {
              console.error('[BVC] Failed to return disc to player:', err);
            }
          }
        } else {
          // Try to insert disc from player's hand
          const equippable = player.getComponent('minecraft:equippable');
          if (!equippable) return;

          const mainHand = equippable.getEquipment(EquipmentSlot.Mainhand);
          if (!mainHand) return;

          const isBvcDisc = mainHand.getDynamicProperty('bvc:is_bvc_disc');
          if (!isBvcDisc) return;

          const audioId = mainHand.getDynamicProperty('bvc:audio_id') as
            | string
            | undefined;
          if (!audioId) return;

          // Insert disc and start playback immediately
          audioManager.insertDisc(
            locationKey,
            audioId,
            block.dimension.id,
            block.x,
            block.y,
            block.z,
            worldUuid
          );
          setBlockPlaying(block, true);

          // Remove disc from player's hand
          equippable.setEquipment(EquipmentSlot.Mainhand, undefined);
        }
      },

      onPlayerBreak(
        e: BlockComponentPlayerBreakEvent,
        _params: CustomComponentParameters
      ) {
        const block = e.block;
        const worldUuid = getWorldUuid();
        const locationKey = audioManager.locationKey(
          worldUuid,
          block.x,
          block.y,
          block.z
        );

        if (audioManager.hasDisc(locationKey)) {
          // Don't call setBlockPlaying here — block is already air after break
          audioManager.onBlockDestroyed(locationKey);
          audioManager.killMarkers(
            block.dimension,
            block.x,
            block.y,
            block.z
          );
        }
      },

      onTick(
        e: BlockComponentTickEvent,
        _params: CustomComponentParameters
      ) {
        const block = e.block;
        const worldUuid = getWorldUuid();
        const locationKey = audioManager.locationKey(
          worldUuid,
          block.x,
          block.y,
          block.z
        );

        const hasDisc = audioManager.hasDisc(locationKey);
        const playingState = block.permutation.getState('bvc:playing' as any);

        // If no disc is loaded, ensure redstone is OFF.
        if (!hasDisc) {
          if (playingState) {
            console.warn(
              `[BVC] onTick: stale bvc:playing=true at (${block.x},${block.y},${block.z}), forcing false`
            );
            setBlockPlaying(block, false);

            // Verify it actually changed
            const after = block.permutation.getState('bvc:playing' as any);
            console.warn(`[BVC] onTick: after setBlockPlaying(false), bvc:playing=${after}`);
          }
        } else {
          return; // disc loaded, nothing to do
        }

        // Only check hopper above for input.
        // Block's own inventory is the OUTPUT buffer (for hopper extraction),
        // so we never pull from it here to avoid an infinite re-grab loop.
        const audioId = pullDiscFromAdjacentHopper(block);

        if (!audioId) return;

        console.info(
          `[BVC] onTick: detected disc audioId=${audioId} at (${block.x},${block.y},${block.z})`
        );

        audioManager.insertDisc(
          locationKey,
          audioId,
          block.dimension.id,
          block.x,
          block.y,
          block.z,
          worldUuid
        );
        setBlockPlaying(block, true);
      },
    });
  });

  // Restore disc state from previous session
  system.run(() => {
    console.info('[BVC] Restoring disc state...');
    audioManager.restore();
  });
}
