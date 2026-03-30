import { system, EquipmentSlot, ItemStack } from '@minecraft/server';
import type {
  Block,
  BlockComponentPlayerInteractEvent,
  BlockComponentPlayerBreakEvent,
  BlockComponentTickEvent,
  CustomComponentParameters,
} from '@minecraft/server';
import { Coordinates } from '../dto';
import { AudioPlayerManager } from './player_manager';

// Adjacent input positions and the hopper facing_direction required to point at our block.
// facing_direction: 0=down, 2=north(-z), 3=south(+z), 4=west(-x), 5=east(+x)
const INPUT_NEIGHBORS: { x: number; y: number; z: number; facing: number }[] = [
  { x: 0, y: 1, z: 0, facing: 0 },
  { x: 1, y: 0, z: 0, facing: 4 },
  { x: -1, y: 0, z: 0, facing: 5 },
  { x: 0, y: 0, z: 1, facing: 2 },
  { x: 0, y: 0, z: -1, facing: 3 },
];

export class AudioComponentRegistry {
  private audioManager: AudioPlayerManager;
  private getWorldUuid: () => string;

  constructor(audioManager: AudioPlayerManager, getWorldUuid: () => string) {
    this.audioManager = audioManager;
    this.getWorldUuid = getWorldUuid;
  }

  register(): void {
    system.beforeEvents.startup.subscribe((event) => {
      event.blockComponentRegistry.registerCustomComponent('bvc:audio_player', {
        onPlayerInteract: (
          e: BlockComponentPlayerInteractEvent,
          _params: CustomComponentParameters
        ) => {
          this.onPlayerInteract(e);
        },

        onPlayerBreak: (
          e: BlockComponentPlayerBreakEvent,
          _params: CustomComponentParameters
        ) => {
          this.onPlayerBreak(e);
        },

        onTick: (
          e: BlockComponentTickEvent,
          _params: CustomComponentParameters
        ) => {
          this.onTick(e);
        },
      });
    });

    system.run(() => {
      console.info('[BVC] Restoring disc state...');
      this.audioManager.restore();
    });
  }

  private onPlayerInteract(e: BlockComponentPlayerInteractEvent): void {
    const block = e.block;
    const player = e.player;
    if (!player) return;

    const worldUuid = this.getWorldUuid();
    const coordinates = new Coordinates(block.x, block.y, block.z);
    const locationKey = this.audioManager.locationKey(worldUuid, coordinates);

    if (this.audioManager.hasDisc(locationKey)) {
      const audioId = this.audioManager.ejectDisc(locationKey);
      this.audioManager.killMarkers(block.dimension, coordinates);
      this.setBlockPlaying(block, false);

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

      this.audioManager.insertDisc(
        locationKey,
        audioId,
        block.dimension.id,
        coordinates,
        worldUuid
      );
      this.setBlockPlaying(block, true);

      equippable.setEquipment(EquipmentSlot.Mainhand, undefined);
    }
  }

  private onPlayerBreak(e: BlockComponentPlayerBreakEvent): void {
    const block = e.block;
    const worldUuid = this.getWorldUuid();
    const coordinates = new Coordinates(block.x, block.y, block.z);
    const locationKey = this.audioManager.locationKey(worldUuid, coordinates);

    if (this.audioManager.hasDisc(locationKey)) {
      this.audioManager.onBlockDestroyed(locationKey);
      this.audioManager.killMarkers(block.dimension, coordinates);
    }
  }

  private onTick(e: BlockComponentTickEvent): void {
    const block = e.block;
    const worldUuid = this.getWorldUuid();
    const coordinates = new Coordinates(block.x, block.y, block.z);
    const locationKey = this.audioManager.locationKey(worldUuid, coordinates);

    const hasDisc = this.audioManager.hasDisc(locationKey);
    const playingState = block.permutation.getState('bvc:playing' as any);

    if (!hasDisc) {
      if (playingState) {
        console.warn(
          `[BVC] onTick: stale bvc:playing=true at (${block.x},${block.y},${block.z}), forcing false`
        );
        this.setBlockPlaying(block, false);
      }
    } else {
      return;
    }

    const audioId = this.pullDiscFromAdjacentHopper(block);
    if (!audioId) return;

    console.info(
      `[BVC] onTick: detected disc audioId=${audioId} at (${block.x},${block.y},${block.z})`
    );

    this.audioManager.insertDisc(
      locationKey,
      audioId,
      block.dimension.id,
      coordinates,
      worldUuid
    );
    this.setBlockPlaying(block, true);
  }

  private setBlockPlaying(block: Block, playing: boolean): void {
    try {
      block.setPermutation(
        block.permutation.withState('bvc:playing' as any, playing)
      );
    } catch (e) {
      console.error('[BVC] Failed to set bvc:playing state:', e);
    }
  }

  private pullDiscFromAdjacentHopper(block: Block): string | null {
    for (const n of INPUT_NEIGHBORS) {
      try {
        const neighbor = block.dimension.getBlock({
          x: block.x + n.x,
          y: block.y + n.y,
          z: block.z + n.z,
        });
        if (!neighbor || !neighbor.typeId.includes('hopper')) continue;

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
        // Skip neighbor if unreadable
      }
    }
    return null;
  }
}
