import { Coordinates } from './coordinates';

/**
 * Request to start audio playback at specific world coordinates.
 */
export class AudioPlayRequest {
  constructor(
    public readonly audio_file_id: string,
    public readonly coordinates: Coordinates,
    public readonly dimension: string,
    public readonly world_uuid: string
  ) {}

  toJSON(): object {
    return {
      audio_file_id: this.audio_file_id,
      game: {
        game: 'minecraft',
        coordinates: this.coordinates.toJSON(),
        dimension: this.dimension,
        world_uuid: this.world_uuid,
      },
    };
  }
}

/**
 * Response from a successful audio play request.
 */
export interface AudioEventResponse {
  event_id: string;
  duration_ms: number;
}
