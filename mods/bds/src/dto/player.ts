import { Player as MinecraftPlayer } from '@minecraft/server';
import { Coordinates } from './coordinates';
import { Orientation } from './orientation';

export class Player {
  constructor(
    public readonly name: string,
    public readonly dimension: string,
    public readonly coordinates: Coordinates,
    public readonly deafen: boolean,
    public readonly orientation: Orientation
  ) {}

  static fromMinecraftPlayer(player: MinecraftPlayer): Player {
    return new Player(
      player.name,
      player.dimension.id.replace('minecraft:', ''),
      Coordinates.fromMinecraftLocation(player.location),
      player.isSneaking,
      Orientation.fromMinecraftRotation(player.getRotation())
    );
  }

  toJSON() {
    return {
      name: this.name,
      dimension: this.dimension,
      coordinates: this.coordinates.toJSON(),
      deafen: this.deafen,
      orientation: this.orientation.toJSON()
    };
  }
}
