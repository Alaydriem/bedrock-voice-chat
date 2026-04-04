import { Player as MinecraftPlayer, GameMode } from '@minecraft/server';
import { Coordinates } from './coordinates';
import { Orientation } from './orientation';
import { Dimension } from './dimension';

export class Player {
  constructor(
    public readonly name: string,
    public readonly dimension: string,
    public readonly coordinates: Coordinates,
    public readonly deafen: boolean,
    public readonly orientation: Orientation,
    public readonly spectator: boolean = false,
    public readonly world_uuid: string | undefined = undefined
  ) {}

  static fromMinecraftPlayer(player: MinecraftPlayer, worldUuid?: string): Player {
    return new Player(
      player.name,
      player.dimension.id.replace('minecraft:', ''),
      Coordinates.fromMinecraftLocation(player.location),
      player.isSneaking,
      Orientation.fromMinecraftRotation(player.getRotation()),
      player.getGameMode() === GameMode.Spectator,
      worldUuid
    );
  }

  /**
   * Create a player DTO with death dimension override.
   * Dead players are placed at origin (0,0,0) in the "death" dimension.
   */
  static fromMinecraftPlayerDead(player: MinecraftPlayer, worldUuid?: string): Player {
    return new Player(
      player.name,
      Dimension.DEATH,
      new Coordinates(0, 0, 0),
      player.isSneaking,
      Orientation.fromMinecraftRotation(player.getRotation()),
      false,
      worldUuid
    );
  }

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
      worldUuid
    );
  }

  toJSON() {
    return {
      name: this.name,
      dimension: this.dimension,
      coordinates: this.coordinates.toJSON(),
      deafen: this.deafen,
      orientation: this.orientation.toJSON(),
      spectator: this.spectator,
      ...(this.world_uuid && { world_uuid: this.world_uuid }),
    };
  }
}
