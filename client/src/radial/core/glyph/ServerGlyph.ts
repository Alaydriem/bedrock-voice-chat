import { Color } from "../color/Color";
import { Hash } from "../math/Hash";
import { MarkData } from "../mark/MarkData";

export interface Glyph {
  /** Colour from a column of the mark. */
  hue: string;
  /** Which column, so a caller can reference the same identity elsewhere. */
  hueIndex: number;
  /** 5x5 grid, row-major, mirrored about the centre column. */
  bits: readonly boolean[];
}

/**
 * A server's identity, derived from its name.
 *
 * No upload, no avatar service, no default grey box. A hostname produces a hue
 * from a column of the mark and a mirrored block pattern, so every server looks
 * distinct, looks like it belongs to this product, and looks the same on every
 * client that knows its name.
 */
export class ServerGlyph {
  static readonly GRID = 5;

  static of(name: string): Glyph {
    const h = Hash.fnv1a(name);
    const hueIndex = h % MarkData.COLS;
    const bits: boolean[] = new Array(ServerGlyph.GRID * ServerGlyph.GRID).fill(false);
    for (let r = 0; r < ServerGlyph.GRID; r++) {
      for (let c = 0; c < 3; c++) {
        if (((h >> (r * 3 + c)) & 1) === 0) continue;
        bits[r * ServerGlyph.GRID + c] = true;
        if (c < 2) bits[r * ServerGlyph.GRID + (4 - c)] = true;
      }
    }
    return { hue: MarkData.hueAt(hueIndex), hueIndex, bits };
  }

  /**
   * @param prog 0 to 1, how much of the pattern has landed. Below 1 the glyph
   *   draws in, which is what makes a server list assemble rather than appear.
   */
  static draw(x: CanvasRenderingContext2D, name: string, size: number, prog = 1): void {
    const glyph = ServerGlyph.of(name);
    const cell = Math.floor(size / ServerGlyph.GRID);
    const pad = (size - cell * ServerGlyph.GRID) / 2;

    x.fillStyle = Color.rgba(glyph.hue, 0.16);
    x.fillRect(0, 0, size, size);

    const total = ServerGlyph.GRID * ServerGlyph.GRID;
    const shown = Math.ceil(prog * total);
    x.fillStyle = glyph.hue;
    for (let i = 0; i < total; i++) {
      if (i >= shown) break;
      if (!glyph.bits[i]) continue;
      const r = Math.floor(i / ServerGlyph.GRID);
      const c = i % ServerGlyph.GRID;
      x.fillRect(pad + c * cell, pad + r * cell, cell - 1, cell - 1);
    }
  }
}
