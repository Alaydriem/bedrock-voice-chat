import { Observable } from '@minecraft/server-ui';

// Mirrors common's QueryState (the acting player's own audio-control state) — the JSON
// GET /api/state returns and the no-net !bvcs: reverse-ride carries. This mod is
// standalone (no shared types with common) by design; keep these fields in sync.
export interface QueryState {
  id: string;
  muted: boolean;
  deafened: boolean;
  recording: boolean;
  current_group: string | null;
}

// Mirrors common's PlayerPreference: the owner's local setting for one target player.
export interface PlayerPreference {
  owner: string;
  target: string;
  volume: number;
  muted: boolean;
}

// Reactive backing store the control panel binds to. Holds the acting player's own
// audio-control state as DDUI Observables plus their per-target preferences. Fed by
// the net poll (StateSource) or the no-net !bvcs: listener — never written directly by
// the panel for self-state: the panel sends an action and lets the feed reflect it
// back, keeping the desktop app / Stream-Deck / panel surfaces consistent.
export class StateCache {
  readonly status = Observable.create<string>('Syncing…');
  readonly muted = Observable.create<boolean>(false, { clientWritable: true });
  readonly deafened = Observable.create<boolean>(false, {
    clientWritable: true,
  });
  readonly recording = Observable.create<boolean>(false);
  // True until the first snapshot lands — the panel binds its controls'
  // `disabled` to this so an unconfirmed on/off is never actionable.
  readonly controlsLocked = Observable.create<boolean>(true);
  // True while the player has no group — the panel disables Leave on it.
  readonly noGroup = Observable.create<boolean>(true);

  private syncing = false;
  private hasSnapshot = false;
  private currentGroup: string | null = null;
  private readonly preferences = new Map<string, PlayerPreference>();
  private readonly prefListeners = new Set<
    (prefs: PlayerPreference[]) => void
  >();

  // While true, an Observable change came from a server snapshot, not the player —
  // the panel's subscribe handlers must NOT echo it back as an action (no feedback
  // loop). `setData` notifies subscribers synchronously, so this brackets safely.
  get isSyncing(): boolean {
    return this.syncing;
  }

  get synced(): boolean {
    return this.hasSnapshot;
  }

  get group(): string | null {
    return this.currentGroup;
  }

  applyQueryState(qs: QueryState): void {
    this.syncing = true;
    this.hasSnapshot = true;
    this.muted.setData(qs.muted);
    this.deafened.setData(qs.deafened);
    this.recording.setData(qs.recording);
    this.currentGroup = qs.current_group;
    this.noGroup.setData(qs.current_group === null);
    this.status.setData(this.groupStatusLine());
    this.controlsLocked.setData(false);
    this.syncing = false;
  }

  applyPreferences(prefs: PlayerPreference[]): void {
    if (prefs.length === 0) {
      return;
    }
    for (const p of prefs) {
      this.preferences.set(p.target, p);
    }
    this.syncing = true;
    for (const listener of this.prefListeners) {
      listener(prefs);
    }
    this.syncing = false;
  }

  // Notifies an open volumes view when fresh preferences arrive from a feed.
  // Returns the unsubscribe function.
  onPreferences(listener: (prefs: PlayerPreference[]) => void): () => void {
    this.prefListeners.add(listener);
    return () => this.prefListeners.delete(listener);
  }

  preferenceFor(target: string): PlayerPreference | undefined {
    return this.preferences.get(target);
  }

  adjustedTargets(): string[] {
    return [...this.preferences.keys()];
  }

  private groupStatusLine(): string {
    return this.currentGroup ? `Group: ${this.currentGroup}` : 'No group';
  }
}
