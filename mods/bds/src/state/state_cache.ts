import { ObservableBoolean, ObservableString } from '@minecraft/server-ui';

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

// The self-state fields a player can change from the panel, and therefore the
// fields a stale snapshot could flap back mid round trip.
export type SelfStateField = 'muted' | 'deafened' | 'recording';

// How long a player-initiated change shadows contradicting snapshots: covers the
// action round trip (delivery + client apply + its ~200ms report debounce) plus a
// full net-poll cycle, with margin.
const PENDING_TTL_MS = 3000;

interface PendingValue<T> {
  value: T;
  expiresAt: number;
}

// Reactive backing store the control panel binds to. Holds the acting player's own
// audio-control state as DDUI Observables plus their per-target preferences. Fed by
// the net poll (StateSource) or the no-net !bvcs: listener. Player-initiated
// changes — self-state via markPending, per-target preferences via
// markVolumePending/markHeardPending — are applied optimistically and shadowed
// against contradicting snapshots until the change's round trip (delivery +
// client apply + its report debounce) can plausibly have completed, so an
// in-flight stale poll cannot flap a control or yank a slider back.
export class StateCache {
  readonly status = new ObservableString('Syncing…');
  readonly muted = new ObservableBoolean(false, { clientWritable: true });
  readonly deafened = new ObservableBoolean(false, {
    clientWritable: true,
  });
  readonly recording = new ObservableBoolean(false);
  // True until the first snapshot lands — the panel binds its controls'
  // `disabled` to this so an unconfirmed on/off is never actionable.
  readonly controlsLocked = new ObservableBoolean(true);
  // True while the player has no group — the panel disables Leave on it.
  readonly noGroup = new ObservableBoolean(true);

  private syncing = false;
  private hasSnapshot = false;
  private currentGroup: string | null = null;
  private readonly pendingChanges = new Map<SelfStateField, PendingValue<boolean>>();
  // Per-target preference shadows, in PlayerPreference units (volume 0..1 gain,
  // heard = !muted) — the volumes view's sliders race the preference poll the
  // same way the panel toggles race the state poll.
  private readonly pendingVolumes = new Map<string, PendingValue<number>>();
  private readonly pendingHeard = new Map<string, PendingValue<boolean>>();
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

  // Records a player-initiated change and applies it optimistically. Snapshots
  // that contradict it (an in-flight poll, a pre-echo cache read) are held off
  // until the TTL lapses — snapshots carry no ordering, so a matching value
  // cannot be distinguished from a coincidence and never lifts the shadow early.
  markPending(field: SelfStateField, value: boolean): void {
    this.pendingChanges.set(field, {
      value,
      expiresAt: Date.now() + PENDING_TTL_MS,
    });
    const observable = this.observableFor(field);
    if (observable.getData() !== value) {
      observable.setData(value);
    }
  }

  applyQueryState(qs: QueryState): void {
    this.syncing = true;
    this.hasSnapshot = true;
    this.applySelfState('muted', this.muted, qs.muted);
    this.applySelfState('deafened', this.deafened, qs.deafened);
    this.applySelfState('recording', this.recording, qs.recording);
    this.currentGroup = qs.current_group;
    this.noGroup.setData(qs.current_group === null);
    this.status.setData(this.groupStatusLine());
    this.controlsLocked.setData(false);
    this.syncing = false;
  }

  private applySelfState(
    field: SelfStateField,
    observable: ObservableBoolean,
    snapshotValue: boolean,
  ): void {
    const pending = this.pendingChanges.get(field);
    if (pending) {
      if (Date.now() < pending.expiresAt) {
        if (snapshotValue !== pending.value) {
          // A stale snapshot (or a client-side failure the expiry will
          // surface): the player's choice holds until the window lapses.
          return;
        }
      } else {
        this.pendingChanges.delete(field);
      }
    }
    observable.setData(snapshotValue);
  }

  private observableFor(field: SelfStateField): ObservableBoolean {
    switch (field) {
      case 'muted':
        return this.muted;
      case 'deafened':
        return this.deafened;
      case 'recording':
        return this.recording;
    }
  }

  applyPreferences(prefs: PlayerPreference[]): void {
    if (prefs.length === 0) {
      return;
    }
    const shadowed = prefs.map((p) => this.shadowedPreference(p));
    for (const p of shadowed) {
      this.preferences.set(p.target, p);
    }
    this.syncing = true;
    for (const listener of this.prefListeners) {
      listener(shadowed);
    }
    this.syncing = false;
  }

  // Records a player-initiated volume change (0..1 gain) and applies it to the
  // stored preference optimistically, so a reopened volumes view seeds from the
  // player's choice rather than a not-yet-confirmed server value.
  markVolumePending(target: string, volume: number): void {
    this.pendingVolumes.set(target, {
      value: volume,
      expiresAt: Date.now() + PENDING_TTL_MS,
    });
    this.upsertPreference(target, { volume });
  }

  markHeardPending(target: string, heard: boolean): void {
    this.pendingHeard.set(target, {
      value: heard,
      expiresAt: Date.now() + PENDING_TTL_MS,
    });
    this.upsertPreference(target, { muted: !heard });
  }

  // The volumes view's commit baseline must not be rewritten by a feed apply
  // that merely reflected our own shadow — that would suppress the real send.
  hasPendingVolume(target: string): boolean {
    const pending = this.pendingVolumes.get(target);
    return pending !== undefined && Date.now() < pending.expiresAt;
  }

  private upsertPreference(
    target: string,
    change: { volume?: number; muted?: boolean },
  ): void {
    const existing = this.preferences.get(target);
    this.preferences.set(target, {
      owner: existing?.owner ?? '',
      target,
      volume: change.volume ?? existing?.volume ?? 1,
      muted: change.muted ?? existing?.muted ?? false,
    });
  }

  // While a target's shadow is unexpired, its pending fields override whatever
  // the snapshot claims (a contradiction is a stale poll or a failure the
  // expiry will surface); expired shadows are dropped.
  private shadowedPreference(pref: PlayerPreference): PlayerPreference {
    const now = Date.now();
    let out = pref;
    const volume = this.pendingVolumes.get(pref.target);
    if (volume) {
      if (now < volume.expiresAt) {
        out = { ...out, volume: volume.value };
      } else {
        this.pendingVolumes.delete(pref.target);
      }
    }
    const heard = this.pendingHeard.get(pref.target);
    if (heard) {
      if (now < heard.expiresAt) {
        out = { ...out, muted: !heard.value };
      } else {
        this.pendingHeard.delete(pref.target);
      }
    }
    return out;
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
