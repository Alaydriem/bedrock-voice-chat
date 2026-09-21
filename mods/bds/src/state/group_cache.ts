export interface GroupRow {
  id: string;
  name: string;
}

/// The server's group list as the panel last saw it.
///
/// One per server, not per player: every player sees the same groups. A `gl` header
/// starts a new list, so rows from an earlier request that arrive afterwards are
/// discarded rather than mixed into the new one.
export class GroupCache {
  private pending: GroupRow[] = [];
  private settled: GroupRow[] = [];
  private expected = 0;
  private headerSeen = false;
  private listeners = new Set<() => void>();

  beginList(count: number): void {
    this.pending = [];
    this.expected = count;
    this.headerSeen = true;
    if (count === 0) {
      this.settle();
    }
  }

  add(row: GroupRow): void {
    this.pending.push(row);
    if (this.pending.length >= this.expected) {
      this.settle();
    }
  }

  // Fewer rows arrived than the header promised. The rows that did arrive are more
  // useful than none, so they settle as they stand.
  settlePartial(): void {
    if (this.headerSeen) {
      this.settle();
    }
  }

  rows(): GroupRow[] {
    return [...this.settled];
  }

  // The name behind a share code, for the panel's status line. Null while the
  // list has not arrived, which the caller renders as the code itself.
  nameFor(id: string): string | null {
    return this.settled.find((row) => row.id === id)?.name ?? null;
  }

  // Whether any header has been seen. False means the proxy never answered, which
  // the page reports as unavailable rather than as an empty server.
  answered(): boolean {
    return this.headerSeen;
  }

  onChange(listener: () => void): () => void {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  }

  private settle(): void {
    this.settled = [...this.pending];
    for (const listener of this.listeners) {
      listener();
    }
  }
}
