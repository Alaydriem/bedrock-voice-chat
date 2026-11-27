import { Player as MinecraftPlayer, GameMode } from '@minecraft/server';
import { Coordinates } from './coordinates';
import { Orientation } from './orientation';
import { Dimension } from './dimension';

export class Player {
  constructor(
    public readonly name: string,
    public readonly dimension: string,
    public readonly coordinates: Coordinates,
    public readonly whispering: boolean,
    public readonly orientation: Orientation,
    public readonly spectator: boolean = false,
    public readonly world_uuid: string | undefined = undefined,
  ) {}

  static fromMinecraftPlayer(
    player: MinecraftPlayer,
    worldUuid?: string,
  ): Player {
    return new Player(
      player.name,
      player.dimension.id.replace('minecraft:', ''),
      Coordinates.fromMinecraftLocation(player.location),
      player.isSneaking,
      Orientation.fromMinecraftRotation(player.getRotation()),
      player.getGameMode() === GameMode.Spectator,
      worldUuid,
    );
  }

  /**
   * Create a player DTO for a dead player.
   * The player keeps their dimension and position and is marked as a spectator.
   */
  static fromMinecraftPlayerDead(
    player: MinecraftPlayer,
    worldUuid?: string,
  ): Player {
    return new Player(
      player.name,
      player.dimension.id.replace('minecraft:', ''),
      Coordinates.fromMinecraftLocation(player.location),
      false,
      Orientation.fromMinecraftRotation(player.getRotation()),
      true,
      worldUuid,
    );
  }

  /**
   * Create a phantom player DTO for a player who has disconnected.
   * Uses the unrouted dimension, extreme coordinates, and spectator=true to guarantee
   * the server's proximity checks reject all audio routing for this player.
   */
  static fromDisconnectedPlayer(name: string, worldUuid?: string): Player {
    return new Player(
      name,
      Dimension.UNROUTED,
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
      dimension: this.dimension,
      coordinates: this.coordinates.toJSON(),
      // Keyed `deafen` on the wire: BVC servers of either version read that key.
      deafen: this.whispering,
      orientation: this.orientation.toJSON(),
      spectator: this.spectator,
      ...(this.world_uuid && { world_uuid: this.world_uuid }),
    };
  }
}
