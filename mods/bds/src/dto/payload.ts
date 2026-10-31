import type { Player as MinecraftPlayer } from '@minecraft/server';
import type { Player } from './player';
import { PlayerFactory } from './player_factory';

export class Payload {
  constructor(
    public readonly game: string = 'minecraft',
    public readonly players: Player[],
  ) {}

  /**
   * Create a payload from Minecraft players.
   * @param players Array of Minecraft players
   * @param deadPlayers Set of player IDs who are currently dead
   * @param worldUuid Optional world UUID for multi-world isolation
   */
  static fromPlayers(
    players: MinecraftPlayer[],
    deadPlayers: Set<string> = new Set(),
    worldUuid?: string,
  ): Payload {
    const playerDtos = players.map((p) =>
      deadPlayers.has(p.id)
        ? PlayerFactory.fromMinecraftPlayerDead(p, worldUuid)
        : PlayerFactory.fromMinecraftPlayer(p, worldUuid),
    );
    return new Payload('minecraft', playerDtos);
  }

  toJSON() {
    return {
      game: this.game,
      players: this.players.map((p) => p.toJSON()),
    };
  }

  toJSONString(): string {
    return JSON.stringify(this.toJSON());
  }
}
