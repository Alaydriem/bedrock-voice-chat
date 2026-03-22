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
import type { AudioPlayerState } from './player_state';
import type { StoredDisc } from './stored_disc';

const STORAGE_KEY = 'bvc:active_discs';

export class AudioPlayerManager {
  private readonly players = new Map<string, AudioPlayerState>();

  constructor(
    private readonly serverUrl: string,
    private readonly accessToken: string
  ) {}

  locationKey(worldUuid: string, coordinates: Coordinates): string {
    return `${worldUuid}:${coordinates.x}:${coordinates.y}:${coordinates.z}`;
  }

  insertDisc(
    locationKey: string,
    audioId: string,
    dimensionId: string,
    coordinates: Coordinates,
    worldUuid: string
  ): void {
    const state: AudioPlayerState = {
      audioId,
      eventId: null,
      isPlaying: false,
      dimensionId,
      coordinates,
      worldUuid,
      autoEjectRunId: null,
    };
    this.players.set(locationKey, state);
    this.persistState();

    this.startPlayback(locationKey, state);
  }

  ejectDisc(locationKey: string): string | undefined {
    const state = this.players.get(locationKey);
    if (!state) return undefined;

    const audioId = state.audioId;

    if (state.autoEjectRunId !== null) {
      system.clearRun(state.autoEjectRunId);
      state.autoEjectRunId = null;
    }

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
          coordinates: new Coordinates(
            disc.coordinates.x,
            disc.coordinates.y,
            disc.coordinates.z
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
    if (!state) return;

    if (state.autoEjectRunId !== null) {
      system.clearRun(state.autoEjectRunId);
    }

    if (state.isPlaying && state.eventId) {
      this.stopPlayback(state);
    }

    this.dropDisc(state);

    this.players.delete(locationKey);
    this.persistState();
  }

  killMarkers(dimension: Dimension, coordinates: Coordinates): void {
    try {
      const entities = dimension.getEntitiesAtBlockLocation({
        x: coordinates.x,
        y: coordinates.y,
        z: coordinates.z,
      });
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
      state.coordinates,
      state.dimensionId.replace('minecraft:', ''),
      state.worldUuid
    );

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
          state.isPlaying = true;
          state.eventId = data.event_id;

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

  private autoEject(locationKey: string): void {
    const state = this.players.get(locationKey);
    if (!state) return;

    state.isPlaying = false;
    state.eventId = null;
    state.autoEjectRunId = null;

    this.setBlockPlaying(state, false);

    if (!this.pushDiscToContainer(state)) {
      this.dropDisc(state);
    }

    this.players.delete(locationKey);
    this.persistState();
  }

  private dropDisc(state: AudioPlayerState): void {
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
    } catch (e) {
      console.error('[BVC] Failed to drop disc:', e);
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
      if (!block) return;

      block.setPermutation(
        block.permutation.withState('bvc:playing' as any, playing)
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
      };
    }
    world.setDynamicProperty(STORAGE_KEY, JSON.stringify(entries));
  }
}
