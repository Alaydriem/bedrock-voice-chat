/**
 * A dimension as Bedrock reports it: a vanilla id, an add-on's custom id, or the
 * synthetic "death" dimension BVC parks dead and disconnected players in.
 *
 * A custom id is sent unchanged: the BVC server hashes it, so every path that names one
 * dimension must send the same string. A vanilla id is sent in its bare form, the only
 * form a server older than this pack accepts.
 */
export class Dimension {
  static readonly DEATH = new Dimension('death');

  private static readonly MINECRAFT_PREFIX = 'minecraft:';

  private static readonly VANILLA = new Set([
    'minecraft:overworld',
    'minecraft:nether',
    'minecraft:the_end',
  ]);

  private constructor(public readonly id: string) {}

  static of(id: string): Dimension {
    return new Dimension(id);
  }

  isCustom(): boolean {
    return !Dimension.VANILLA.has(this.id) && this.id !== Dimension.DEATH.id;
  }

  equals(other: Dimension): boolean {
    return this.id === other.id;
  }

  toApiString(): string {
    if (Dimension.VANILLA.has(this.id)) {
      return this.id.slice(Dimension.MINECRAFT_PREFIX.length);
    }
    return this.id;
  }
}
