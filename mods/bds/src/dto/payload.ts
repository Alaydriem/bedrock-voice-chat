import { Player as MinecraftPlayer } from '@minecraft/server';
import { Player } from './player';

export class Payload {
  constructor(
    public readonly game: string = 'minecraft',
    public readonly players: Player[]
  ) {}

  /**
   * Create a payload from Minecraft players.
   * @param players Array of Minecraft players
   * @param deadPlayers Set of player IDs who are currently dead
   */
  static fromPlayers(players: MinecraftPlayer[], deadPlayers: Set<string> = new Set()): Payload {
    const playerDtos = players.map(p =>
      deadPlayers.has(p.id)
        ? Player.fromMinecraftPlayerDead(p)
        : Player.fromMinecraftPlayer(p)
    );
    return new Payload('minecraft', playerDtos);
  }

  toJSON() {
    return {
      game: this.game,
      players: this.players.map(p => p.toJSON())
    };
  }

  toJSONString(): string {
    return JSON.stringify(this.toJSON());
  }
}
