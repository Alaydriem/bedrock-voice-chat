// Temporary, deleted in Task 13 alongside ChatProbe.
//
// One phase of the transport measurement: a bag of HTTP round-trip samples plus how many
// failed. Three of these — before, during and after a held websocket — are what answer whether
// holding a socket open starves the HTTP requests that share @minecraft/server-net.
export class ProbeTiming {
  private readonly samples: number[] = [];
  private failures = 0;

  constructor(private readonly label: string) {}

  record(ms: number, ok: boolean): void {
    this.samples.push(ms);
    if (!ok) {
      this.failures++;
    }
  }

  get count(): number {
    return this.samples.length;
  }

  get mean(): number {
    if (this.samples.length === 0) {
      return 0;
    }
    let total = 0;
    for (const s of this.samples) {
      total += s;
    }
    return Math.round(total / this.samples.length);
  }

  // The number that matters. A starved request shows up as a long tail well before it shows
  // up in the mean, and the position loop's own timeout is only 1s.
  get p95(): number {
    if (this.samples.length === 0) {
      return 0;
    }
    const sorted = [...this.samples].sort((a, b) => a - b);
    const index = Math.min(sorted.length - 1, Math.floor(sorted.length * 0.95));
    return sorted[index];
  }

  get max(): number {
    let max = 0;
    for (const s of this.samples) {
      if (s > max) {
        max = s;
      }
    }
    return max;
  }

  summary(): string {
    return (
      `${this.label.padEnd(16)} n=${String(this.count).padStart(3)} ` +
      `mean=${String(this.mean).padStart(4)}ms ` +
      `p95=${String(this.p95).padStart(4)}ms ` +
      `max=${String(this.max).padStart(4)}ms ` +
      `fail=${this.failures}`
    );
  }
}
