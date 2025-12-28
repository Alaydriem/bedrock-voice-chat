export class Coordinates {
  constructor(
    public readonly x: number,
    public readonly y: number,
    public readonly z: number
  ) {}

  static fromMinecraftLocation(location: { x: number; y: number; z: number }): Coordinates {
    return new Coordinates(
      location.x,
      location.y,
      location.z
    );
  }

  toJSON() {
    return {
      x: this.x,
      y: this.y,
      z: this.z
    };
  }
}
