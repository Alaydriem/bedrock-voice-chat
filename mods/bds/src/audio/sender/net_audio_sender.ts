import { system } from '@minecraft/server';
import type { Player } from '@minecraft/server';
import { AudioPlayRequest } from '../../dto';
import type { AudioEventResponse } from '../../dto';
import { httpClient } from '../../net';
import type { ServerAdminConfig } from '../../config';
import type { AudioPlayerState } from '../player_state';
import type { AudioSender } from './audio_sender';

export class NetAudioSender implements AudioSender {
  constructor(
    private readonly config: ServerAdminConfig,
    private readonly onAutoEject: (locationKey: string) => void,
    private readonly locationKeyOf: (state: AudioPlayerState) => string,
  ) {}

  async start(state: AudioPlayerState, _actor: Player | null): Promise<void> {
    const request = new AudioPlayRequest(
      state.audioId,
      state.coordinates,
      state.dimensionId.replace('minecraft:', ''),
      state.worldUuid,
    );
    const body = JSON.stringify(request.toJSON());
    const locationKey = this.locationKeyOf(state);

    try {
      const response = await httpClient.request(
        `${this.config.bvcServer}/api/audio/event`,
        'Post',
        body,
        [
          ['Content-Type', 'application/json'],
          ['X-MC-Access-Token', this.config.accessToken],
          ['Accept', 'application/json'],
        ],
        5,
      );

      if (!response) {
        state.isPlaying = false;
        return;
      }

      if (response.status < 200 || response.status >= 300) {
        state.isPlaying = false;
        console.warn(`[BVC] Play request failed: ${response.status}`);
        return;
      }

      const data: AudioEventResponse = JSON.parse(response.body);
      state.isPlaying = true;
      state.eventId = data.event_id;

      const ticks = Math.ceil(data.duration_ms / 50);
      state.autoEjectRunId = system.runTimeout(() => {
        this.onAutoEject(locationKey);
      }, ticks);
    } catch (e) {
      state.isPlaying = false;
      console.error('[BVC] Failed to start playback:', e);
    }
  }

  async stop(state: AudioPlayerState, _actor: Player | null): Promise<void> {
    const eventId = state.eventId;
    if (!eventId) return;

    state.isPlaying = false;
    state.eventId = null;

    try {
      const response = await httpClient.request(
        `${this.config.bvcServer}/api/audio/event/${eventId}`,
        'Delete',
        undefined,
        [
          ['X-MC-Access-Token', this.config.accessToken],
          ['Accept', 'application/json'],
        ],
        5,
      );

      if (!response) return;
      if (response.status >= 200 && response.status < 300) return;

      console.warn(`[BVC] Stop request failed: ${response.status}`);
    } catch (e) {
      console.error('[BVC] Failed to stop playback:', e);
    }
  }
}
