import type { Player } from '@minecraft/server';
import type { AudioPlayerState } from '../player_state';

export interface AudioSender {
  start(state: AudioPlayerState, actor: Player | null): Promise<void>;
  stop(state: AudioPlayerState, actor: Player | null): Promise<void>;
}
