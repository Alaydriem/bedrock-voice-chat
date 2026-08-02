/** One column of the mark: the rows it spans, and its colour. */
export type MarkColumn = readonly [top: number, bottom: number, hex: string];

/**
 * The mark.
 *
 * Ported verbatim from client/src/radial/core/mark/MarkData.ts.
 *
 * `app-logo-transparent.svg` is not a badge. It is 137 blocks on a 23x13 grid
 * where every column is one contiguous vertical span — a waveform envelope
 * running violet to red across the spectrum. That compresses to the 23 triplets
 * below, and one drawing routine renders them linearly (a header, a level
 * meter), radially (the ring), and at any amplitude.
 */
export class MarkData {
  static readonly COLUMNS: readonly MarkColumn[] = [
    [6, 6, '#8239d8'],
    [5, 6, '#8238d8'],
    [3, 7, '#8238d8'],
    [4, 8, '#6a50e9'],
    [5, 10, '#466cf3'],
    [3, 11, '#3d93ed'],
    [1, 8, '#28bae1'],
    [2, 9, '#21d8d8'],
    [4, 8, '#26ddcd'],
    [2, 7, '#34d8a0'],
    [0, 7, '#3bd869'],
    [2, 10, '#6fd846'],
    [4, 11, '#aee236'],
    [6, 12, '#f8e433'],
    [5, 11, '#f8e434'],
    [3, 8, '#f9bf21'],
    [1, 6, '#f99a23'],
    [0, 7, '#f9871d'],
    [3, 9, '#f67414'],
    [5, 10, '#f65021'],
    [4, 8, '#f0422b'],
    [5, 6, '#f8352b'],
    [6, 6, '#f63125'],
  ];

  static readonly COLS = MarkData.COLUMNS.length;
  static readonly ROWS = 13;
  /** The row the waveform collapses onto. */
  static readonly MID = 6;

  /** Width of the mark in CSS px for a given cell size and gap. */
  static width(cell: number, gap: number): number {
    return MarkData.COLS * (cell + gap) - gap;
  }

  /** Height of the mark in CSS px for a given cell size and gap. */
  static height(cell: number, gap: number): number {
    return MarkData.ROWS * (cell + gap) - gap;
  }

  /** The colour of a column, wrapping so any integer is a valid index. */
  static hueAt(index: number): string {
    const i = ((index % MarkData.COLS) + MarkData.COLS) % MarkData.COLS;
    return MarkData.COLUMNS[i]![2];
  }

  /** The gap that holds the mark's proportions at any cell size. */
  static gapFor(cell: number): number {
    return Math.max(1, Math.round(cell * 0.3));
  }

  /** Cell size that makes the mark exactly `boxWidth` wide, gap included. */
  static cellForWidth(boxWidth: number): number {
    return boxWidth / (MarkData.COLS + 0.3 * (MarkData.COLS - 1));
  }
}
