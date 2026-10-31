import { Coordinates } from './coordinates';
import { Orientation } from './orientation';
import { Dimension } from './dimension';

/**
 * One player's position record, as posted to the BVC server.
 *
 * Imports nothing from `@minecraft/*` so it can be tested outside Bedrock; building one
 * from a live player is `PlayerFactory`'s job.
 */
export class Player {
  constructor(
    public readonly name: string,
    public readonly dimension: Dimension,
    public readonly coordinates: Coordinates,
    public readonly whispering: boolean,
    public readonly orientation: Orientation,
    public readonly spectator: boolean = false,
    public readonly world_uuid: string | undefined = undefined,
  ) {}

  /**
   * Create a phantom player DTO for a player who has disconnected.
   * Uses death dimension, extreme coordinates, and spectator=true to guarantee
   * the server's proximity checks reject all audio routing for this player.
   */
  static fromDisconnectedPlayer(name: string, worldUuid?: string): Player {
    return new Player(
      name,
      Dimension.DEATH,
      new Coordinates(-10000, -10000, -10000),
      false,
      new Orientation(0, 0),
      true,
      worldUuid,
    );
  }

  toJSON() {
    return {
      name: this.name,
      dimension: this.dimension.toApiString(),
      coordinates: this.coordinates.toJSON(),
      // Keyed `deafen` on the wire: BVC servers of either version read that key.
      deafen: this.whispering,
      orientation: this.orientation.toJSON(),
      spectator: this.spectator,
      ...(this.world_uuid && { world_uuid: this.world_uuid }),
    };
  }
}
