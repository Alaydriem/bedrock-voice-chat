import { world, system, ItemStack } from '@minecraft/server';
import type { Dimension } from '@minecraft/server';
import {
  http,
  HttpRequest,
  HttpRequestMethod,
  HttpHeader,
} from '@minecraft/server-net';
import { Coordinates, AudioPlayRequest } from '../dto';
import type { AudioEventResponse } from '../dto';

/**
 * State for an audio player block.
 */
interface AudioPlayerState {
  audioId: string;
  eventId: string | null;
  isPlaying: boolean;
  dimensionId: string;
  x: number;
  y: number;
  z: number;
  worldUuid: string;
  /** system.runTimeout ID for auto-eject when playback finishes. */
  autoEjectRunId: number | null;
}

/** Stored entry for world dynamic property persistence. */
interface StoredDisc {
  audioId: string;
  dimensionId: string;
  x: number;
  y: number;
  z: number;
  worldUuid: string;
}

const STORAGE_KEY = 'bvc:active_discs';

/**
 * Manages BVC audio player blocks on BDS.
 *
 * Insert → play (HTTP POST) → auto-eject when done.
 * Manual eject → stop (HTTP DELETE) → return disc.
 */
export class AudioPlayerManager {
  private readonly players = new Map<string, AudioPlayerState>();

  constructor(
    private readonly serverUrl: string,
    private readonly accessToken: string
  ) {}

  locationKey(worldUuid: string, x: number, y: number, z: number): string {
    return `${worldUuid}:${x}:${y}:${z}`;
  }

  /**
   * Insert a disc and immediately start playback.
   */
  insertDisc(
    locationKey: string,
    audioId: string,
    dimensionId: string,
    x: number,
    y: number,
    z: number,
    worldUuid: string
  ): void {
    const state: AudioPlayerState = {
      audioId,
      eventId: null,
      isPlaying: false,
      dimensionId,
      x,
      y,
      z,
      worldUuid,
      autoEjectRunId: null,
    };
    this.players.set(locationKey, state);
    this.persistState();

    this.startPlayback(locationKey, state);
  }

  /**
   * Manually eject a disc. Stops playback and returns disc to player inventory.
   * Returns the audioId so the caller can create the ItemStack.
   */
  ejectDisc(locationKey: string): string | undefined {
    const state = this.players.get(locationKey);
    if (!state) return undefined;

    const audioId = state.audioId;

    // Cancel auto-eject timer
    if (state.autoEjectRunId !== null) {
      system.clearRun(state.autoEjectRunId);
      state.autoEjectRunId = null;
    }

    // Stop playback on server
    if (state.isPlaying && state.eventId) {
      this.stopPlayback(state);
    }

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

  /**
   * Restore disc state from world dynamic properties.
   * Restored discs are idle (not playing). Player can eject and re-insert.
   */
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
        if (this.players.has(key)) continue;

        this.players.set(key, {
          audioId: disc.audioId,
          eventId: null,
          isPlaying: false,
          dimensionId: disc.dimensionId,
          x: disc.x,
          y: disc.y,
          z: disc.z,
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

  /**
   * Handle block destruction. Stops playback and drops disc.
   */
  onBlockDestroyed(locationKey: string): void {
    const state = this.players.get(locationKey);
    if (!state) return;

    if (state.autoEjectRunId !== null) {
      system.clearRun(state.autoEjectRunId);
    }

    if (state.isPlaying && state.eventId) {
      this.stopPlayback(state);
    }

    // Drop disc as item
    this.dropDisc(state);

    this.players.delete(locationKey);
    this.persistState();
  }

  /**
   * Kill marker entities at a block position (cleanup from old versions).
   */
  killMarkers(dimension: Dimension, x: number, y: number, z: number): void {
    try {
      const entities = dimension.getEntitiesAtBlockLocation({ x, y, z });
      for (const entity of entities) {
        if (
          entity.typeId === 'minecraft:marker' &&
          entity.getDynamicProperty('bvc:audio_id')
        ) {
          entity.kill();
        }
      }
    } catch (e) {
      console.error('[BVC] Failed to kill marker entities:', e);
    }
  }

  private startPlayback(locationKey: string, state: AudioPlayerState): void {
    const request = new AudioPlayRequest(
      state.audioId,
      new Coordinates(state.x, state.y, state.z),
      state.dimensionId.replace('minecraft:', ''),
      state.worldUuid
    );

    state.isPlaying = true;

    const body = JSON.stringify(request.toJSON());

    const httpRequest = new HttpRequest(
      `${this.serverUrl}/api/audio/event`
    );
    httpRequest.setBody(body);
    httpRequest.setMethod((HttpRequestMethod as any).Post);
    httpRequest.setHeaders([
      new HttpHeader('Content-Type', 'application/json'),
      new HttpHeader('X-MC-Access-Token', this.accessToken),
      new HttpHeader('Accept', 'application/json'),
    ]);
    httpRequest.setTimeout(5);

    http
      .request(httpRequest)
      .then((response) => {
        if (response.status >= 200 && response.status < 300) {
          const data: AudioEventResponse = JSON.parse(response.body);
          state.eventId = data.event_id;

          // Schedule auto-eject when playback finishes
          const ticks = Math.ceil(data.duration_ms / 50);
          state.autoEjectRunId = system.runTimeout(() => {
            this.autoEject(locationKey);
          }, ticks);
        } else {
          state.isPlaying = false;
          console.warn(`[BVC] Play request failed: ${response.status}`);
        }
      })
      .catch((e) => {
        state.isPlaying = false;
        console.error('[BVC] Failed to start playback:', e);
      });
  }

  private stopPlayback(state: AudioPlayerState): void {
    const eventId = state.eventId;
    if (!eventId) return;

    state.isPlaying = false;
    state.eventId = null;

    const httpRequest = new HttpRequest(
      `${this.serverUrl}/api/audio/event/${eventId}`
    );
    httpRequest.setMethod((HttpRequestMethod as any).Delete);
    httpRequest.setHeaders([
      new HttpHeader('X-MC-Access-Token', this.accessToken),
      new HttpHeader('Accept', 'application/json'),
    ]);
    httpRequest.setTimeout(5);

    http
      .request(httpRequest)
      .then((response) => {
        if (response.status >= 200 && response.status < 300) {
        } else {
          console.warn(`[BVC] Stop request failed: ${response.status}`);
        }
      })
      .catch((e) => {
        console.error('[BVC] Failed to stop playback:', e);
      });
  }

  /**
   * Auto-eject: playback finished naturally.
   * Turns off redstone, then tries to place disc into block inventory or hopper below.
   * Falls back to dropping disc as a world item.
   */
  private autoEject(locationKey: string): void {
    const state = this.players.get(locationKey);
    if (!state) return;

    state.isPlaying = false;
    state.eventId = null;
    state.autoEjectRunId = null;

    // Turn off redstone signal
    this.setBlockPlaying(state, false);

    // Try to place disc into a container, fall back to world drop
    if (!this.pushDiscToContainer(state)) {
      this.dropDisc(state);
    }

    this.players.delete(locationKey);
    this.persistState();
  }

  /**
   * Drop a disc as a world item at the block position.
   */
  private dropDisc(state: AudioPlayerState): void {
    try {
      const dimension = world.getDimension(state.dimensionId);
      const disc = new ItemStack('minecraft:music_disc_5', 1);
      disc.nameTag = `BVC: ${state.audioId}`;
      disc.setDynamicProperty('bvc:audio_id', state.audioId);
      disc.setDynamicProperty('bvc:is_bvc_disc', true);

      dimension.spawnItem(disc, {
        x: state.x + 0.5,
        y: state.y + 1.0,
        z: state.z + 0.5,
      });
    } catch (e) {
      console.error('[BVC] Failed to drop disc:', e);
    }
  }

  /**
   * Set the bvc:playing block state (controls redstone_producer permutation).
   */
  private setBlockPlaying(state: AudioPlayerState, playing: boolean): void {
    try {
      const dimension = world.getDimension(state.dimensionId);
      const block = dimension.getBlock({ x: state.x, y: state.y, z: state.z });
      if (!block) return;

      block.setPermutation(
        block.permutation.withState('bvc:playing' as any, playing)
      );
    } catch (e) {
      console.error('[BVC] Failed to set block playing state:', e);
    }
  }

  /**
   * Try to place a disc into the block's own inventory or a hopper below.
   * Returns true if the disc was placed into a container.
   */
  private pushDiscToContainer(state: AudioPlayerState): boolean {
    try {
      const dimension = world.getDimension(state.dimensionId);
      const disc = new ItemStack('minecraft:music_disc_5', 1);
      disc.nameTag = `BVC: ${state.audioId}`;
      disc.setDynamicProperty('bvc:audio_id', state.audioId);
      disc.setDynamicProperty('bvc:is_bvc_disc', true);

      // Try block's own inventory first (storage_item approach)
      const block = dimension.getBlock({ x: state.x, y: state.y, z: state.z });
      if (block) {
        const inv = block.getComponent('minecraft:inventory') as any;
        if (inv?.container) {
          inv.container.setItem(0, disc);
          return true;
        }
      }

      // Try hopper below
      const below = dimension.getBlock({
        x: state.x,
        y: state.y - 1,
        z: state.z,
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
        x: state.x,
        y: state.y,
        z: state.z,
        worldUuid: state.worldUuid,
      };
    }
    world.setDynamicProperty(STORAGE_KEY, JSON.stringify(entries));
  }
}
