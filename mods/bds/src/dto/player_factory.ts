import { Player as MinecraftPlayer, GameMode } from '@minecraft/server';
import { Coordinates } from './coordinates';
import { Dimension } from './dimension';
import { Orientation } from './orientation';
import { Player } from './player';

/**
 * Builds position records from live Bedrock players.
 */
export class PlayerFactory {
  static fromMinecraftPlayer(player: MinecraftPlayer, worldUuid?: string): Player {
    return new Player(
      player.name,
      Dimension.of(player.dimension.id),
      Coordinates.fromMinecraftLocation(player.location),
      player.isSneaking,
      Orientation.fromMinecraftRotation(player.getRotation()),
      player.getGameMode() === GameMode.Spectator,
      worldUuid,
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
    );
  }
}
