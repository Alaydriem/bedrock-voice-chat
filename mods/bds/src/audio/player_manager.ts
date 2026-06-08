import { world, system, ItemStack } from '@minecraft/server';
import type { Player } from '@minecraft/server';
import { Coordinates } from '../dto';
import type { AudioPlayerState } from './player_state';
import type { StoredDisc } from './stored_disc';
import type { AudioSender } from './sender';

const STORAGE_KEY = 'bvc:active_discs';

export class AudioPlayerManager {
  private readonly players = new Map<string, AudioPlayerState>();
  private sender: AudioSender | null = null;

  setSender(sender: AudioSender): void {
    this.sender = sender;
  }

  locationKey(worldUuid: string, coordinates: Coordinates): string {
    return `${worldUuid}:${coordinates.x}:${coordinates.y}:${coordinates.z}`;
  }

  insertDisc(
    locationKey: string,
    audioId: string,
    dimensionId: string,
    coordinates: Coordinates,
    worldUuid: string,
    actor: Player | null,
  ): void {
    const state: AudioPlayerState = {
      audioId,
      eventId: null,
      isPlaying: true,
      dimensionId,
      coordinates,
      worldUuid,
      autoEjectRunId: null,
    };

    this.players.set(locationKey, state);
    this.persistState();

    void this.sender?.start(state, actor);
  }

  ejectDisc(locationKey: string, actor: Player | null): string | undefined {
    const state = this.players.get(locationKey);
    if (!state) {
      return undefined;
    }

    const audioId = state.audioId;

    if (state.autoEjectRunId !== null) {
      system.clearRun(state.autoEjectRunId);
      state.autoEjectRunId = null;
    }

    void this.sender?.stop(state, actor);

    this.players.delete(locationKey);
    this.persistState();
    return audioId;
  }

  hasDisc(locationKey: string): boolean {
    return this.players.has(locationKey);
  }

  getAudioId(locationKey: string): string | undefined {
    return this.players.get(locationKey)?.audioId;
  }

  isPlaying(locationKey: string): boolean {
    return this.players.get(locationKey)?.isPlaying ?? false;
  }

  restore(): void {
    try {
      const raw = world.getDynamicProperty(STORAGE_KEY);
      if (typeof raw !== 'string') {
        console.info('[BVC] No saved disc state found');
        return;
      }

      const entries: Record<string, StoredDisc> = JSON.parse(raw);
      let restored = 0;

      for (const [key, disc] of Object.entries(entries)) {
        if (this.players.has(key)) {
          continue;
        }

        this.players.set(key, {
          audioId: disc.audioId,
          eventId: null,
          isPlaying: false,
          dimensionId: disc.dimensionId,
          coordinates: new Coordinates(
            disc.coordinates.x,
            disc.coordinates.y,
            disc.coordinates.z,
          ),
          worldUuid: disc.worldUuid,
          autoEjectRunId: null,
        });

        restored++;
        console.info(`[BVC] Restored disc: ${key} audioId=${disc.audioId}`);
      }

      console.info(`[BVC] Restored ${restored} disc(s)`);
    } catch (e) {
      console.error('[BVC] Failed to restore disc state:', e);
    }
  }

  onBlockDestroyed(locationKey: string): void {
    const state = this.players.get(locationKey);
    if (!state) {
      return;
    }

    if (state.autoEjectRunId !== null) {
      system.clearRun(state.autoEjectRunId);
    }

    void this.sender?.stop(state, null);

    this.dropDisc(state);

    this.players.delete(locationKey);
    this.persistState();
  }

  forceEject(locationKey: string): boolean {
    if (!this.players.has(locationKey)) {
      return false;
    }

    this.autoEject(locationKey);
    return true;
  }

  private autoEject(locationKey: string): void {
    const state = this.players.get(locationKey);
    if (!state) {
      return;
    }

    state.autoEjectRunId = null;

    const dimension = world.getDimension(state.dimensionId);
    const block = dimension.getBlock({
      x: state.coordinates.x,
      y: state.coordinates.y,
      z: state.coordinates.z,
    });

    // Chunk not loaded: keep the disc in the jukebox so it is not lost. Mark it
    // finished and let onTick run the deferred eject once the chunk is ticking.
    if (!block) {
      state.isPlaying = false;
      this.persistState();
      return;
    }

    state.isPlaying = false;
    state.eventId = null;

    this.setBlockPlaying(state, false);

    // The chunk is loaded but may not be ticking, in which case spawnItem throws
    // and dropDisc reports failure. Keep the disc rather than deleting it.
    const handed = this.pushDiscToContainer(state) || this.dropDisc(state);
    if (!handed) {
      this.persistState();
      return;
    }

    this.players.delete(locationKey);
    this.persistState();
  }

  private dropDisc(state: AudioPlayerState): boolean {
    try {
      const dimension = world.getDimension(state.dimensionId);
      const disc = new ItemStack('minecraft:music_disc_5', 1);
      disc.nameTag = `BVC: ${state.audioId}`;
      disc.setDynamicProperty('bvc:audio_id', state.audioId);
      disc.setDynamicProperty('bvc:is_bvc_disc', true);

      dimension.spawnItem(disc, {
        x: state.coordinates.x + 0.5,
        y: state.coordinates.y + 1.0,
        z: state.coordinates.z + 0.5,
      });

      return true;
    } catch (e) {
      console.warn('[BVC] Failed to drop disc:', e);
      return false;
    }
  }

  private setBlockPlaying(state: AudioPlayerState, playing: boolean): void {
    try {
      const dimension = world.getDimension(state.dimensionId);
      const block = dimension.getBlock({
        x: state.coordinates.x,
        y: state.coordinates.y,
        z: state.coordinates.z,
      });

      if (!block) {
        return;
      }

      block.setPermutation(
        block.permutation.withState('bvc:playing' as any, playing),
      );
    } catch (e) {
      console.error('[BVC] Failed to set block playing state:', e);
    }
  }

  private pushDiscToContainer(state: AudioPlayerState): boolean {
    try {
      const dimension = world.getDimension(state.dimensionId);
      const disc = new ItemStack('minecraft:music_disc_5', 1);
      disc.nameTag = `BVC: ${state.audioId}`;
      disc.setDynamicProperty('bvc:audio_id', state.audioId);
      disc.setDynamicProperty('bvc:is_bvc_disc', true);

      const block = dimension.getBlock({
        x: state.coordinates.x,
        y: state.coordinates.y,
        z: state.coordinates.z,
      });

      if (block) {
        const inv = block.getComponent('minecraft:inventory') as any;
        if (inv?.container) {
          inv.container.setItem(0, disc);
          return true;
        }
      }

      const below = dimension.getBlock({
        x: state.coordinates.x,
        y: state.coordinates.y - 1,
        z: state.coordinates.z,
      });

      if (below && below.typeId.includes('hopper')) {
        const hopperInv = below.getComponent('minecraft:inventory') as any;
        if (hopperInv?.container) {
          hopperInv.container.addItem(disc);
          return true;
        }
      }
    } catch (e) {
      console.error('[BVC] Failed to push disc to container:', e);
    }
    return false;
  }

  private persistState(): void {
    const entries: Record<string, StoredDisc> = {};
    for (const [key, state] of this.players) {
      entries[key] = {
        audioId: state.audioId,
        dimensionId: state.dimensionId,
        coordinates: state.coordinates.toJSON(),
        worldUuid: state.worldUuid,
        isPlaying: state.isPlaying,
      };
    }
    world.setDynamicProperty(STORAGE_KEY, JSON.stringify(entries));
  }
}
