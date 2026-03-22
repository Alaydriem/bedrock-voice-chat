import type { Coordinates } from '../dto';

export interface AudioPlayerState {
  audioId: string;
  eventId: string | null;
  isPlaying: boolean;
  dimensionId: string;
  coordinates: Coordinates;
  worldUuid: string;
  // system.runTimeout ID for auto-eject when playback finishes
  autoEjectRunId: number | null;
}
