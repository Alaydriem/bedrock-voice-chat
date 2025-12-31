import { Player as MinecraftPlayer } from '@minecraft/server';
import { Player } from './player';

export class Payload {
  constructor(
    public readonly game: string = 'minecraft',
    public readonly players: Player[]
  ) {}

  static fromPlayers(players: MinecraftPlayer[]): Payload {
    const playerDtos = players.map(p => Player.fromMinecraftPlayer(p));
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
