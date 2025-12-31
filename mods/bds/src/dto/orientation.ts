export class Orientation {
  constructor(
    public readonly x: number,
    public readonly y: number
  ) {}

  static fromMinecraftRotation(rotation: { x: number; y: number }): Orientation {
    return new Orientation(
      rotation.x,
      rotation.y
    );
  }

  toJSON() {
    return {
      x: this.x,
      y: this.y
    };
  }
}
