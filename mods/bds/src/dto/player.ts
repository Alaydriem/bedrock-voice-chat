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
    public readonly world_uuid: string | undefined = undefined,
    public readonly player_uuid: string | undefined = undefined
  ) {}

  static fromMinecraftPlayer(player: MinecraftPlayer, worldUuid?: string): Player {
    return new Player(
      player.name,
      player.dimension.id.replace('minecraft:', ''),
      Coordinates.fromMinecraftLocation(player.location),
      player.isSneaking,
      Orientation.fromMinecraftRotation(player.getRotation()),
      player.getGameMode() === GameMode.Spectator,
      worldUuid,
      player.id
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
      worldUuid,
      player.id
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
      ...(this.player_uuid && { player_uuid: this.player_uuid }),
    };
  }
}
