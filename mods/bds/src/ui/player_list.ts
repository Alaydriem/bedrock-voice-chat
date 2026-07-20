import { system, world } from '@minecraft/server';
import type { Player } from '@minecraft/server';
import { CustomForm, Observable } from '@minecraft/server-ui';
import type { ControlSender } from '../control/sender';
import type { StateCache } from '../state/state_cache';

// Players within this many blocks are offered in the volumes list even if the
// owner has never adjusted them; already-adjusted players always appear.
export const NEARBY_RADIUS_BLOCKS = 64;

// Sliders fire on every notch; commit the value only after the drag settles.
const SLIDER_COMMIT_TICKS = 10;

interface VolumeRow {
  target: string;
  heard: Observable<boolean>;
  volume: Observable<number>;
}

export type { VolumeRow };

/// The per-player volumes sub-view: one row per displayed player with a Hear
/// toggle and a 0–100 volume slider, each committing the owner's LOCAL playback
/// preference through the control plane. Rows update live when fresh
/// preferences ride in from a feed.
export class PlayerVolumesView {
  constructor(
    private readonly sender: ControlSender,
    private readonly cache: StateCache,
  ) {}

  // Proximity-scoped list: players within NEARBY_RADIUS_BLOCKS (same dimension,
  // excluding the owner) plus everyone the owner has already adjusted.
  rows(owner: Player): VolumeRow[] {
    const names = new Set<string>();
    for (const other of world.getAllPlayers()) {
      if (other.name === owner.name) {
        continue;
      }
      if (other.dimension.id !== owner.dimension.id) {
        continue;
      }
      const dx = other.location.x - owner.location.x;
      const dy = other.location.y - owner.location.y;
      const dz = other.location.z - owner.location.z;
      if (
        dx * dx + dy * dy + dz * dz <=
        NEARBY_RADIUS_BLOCKS * NEARBY_RADIUS_BLOCKS
      ) {
        names.add(other.name);
      }
    }
    for (const adjusted of this.cache.adjustedTargets()) {
      names.add(adjusted);
    }

    return [...names].sort().map((target) => {
      const pref = this.cache.preferenceFor(target);
      return {
        target,
        heard: Observable.create<boolean>(!(pref?.muted ?? false), {
          clientWritable: true,
        }),
        volume: Observable.create<number>(
          Math.round((pref?.volume ?? 1) * 100),
          { clientWritable: true },
        ),
      };
    });
  }

  async show(owner: Player, rows: VolumeRow[]): Promise<void> {
    const unsubscribes: Array<() => void> = [];
    const pendingCommits = new Map<string, number>();

    const form = CustomForm.create(owner, 'Player volumes');
    if (rows.length === 0) {
      form.label('No players nearby');
    }

    for (const row of rows) {
      form
        .header(row.target)
        .toggle('Hear', row.heard)
        .slider('Volume', row.volume, 0, 100, { step: 5 });

      // Like the panel toggles: the syncing bracket covers synchronous setData
      // dispatch, the last-seen comparison covers asynchronous dispatch, so a
      // feed-applied row update can never loop back into an action.
      let lastHeard = row.heard.getData();
      const heardListener = row.heard.subscribe((on) => {
        if (on === lastHeard) {
          return;
        }
        lastHeard = on;
        if (!this.cache.isSyncing) {
          void this.sender.send(
            { kind: 'hear', target: row.target, on },
            owner,
          );
        }
      });
      unsubscribes.push(() => row.heard.unsubscribe(heardListener));

      let lastCommittedVolume = row.volume.getData();
      const volumeListener = row.volume.subscribe((value) => {
        if (this.cache.isSyncing) {
          lastCommittedVolume = value;
          return;
        }
        // Debounce the drag: only the value that survives the settle window is
        // committed.
        const pending = pendingCommits.get(row.target);
        if (pending !== undefined) {
          system.clearRun(pending);
        }
        pendingCommits.set(
          row.target,
          system.runTimeout(() => {
            pendingCommits.delete(row.target);
            if (value === lastCommittedVolume) {
              return;
            }
            lastCommittedVolume = value;
            void this.sender.send(
              {
                kind: 'volume',
                target: row.target,
                value: Math.max(0, Math.min(100, Math.round(value))),
              },
              owner,
            );
          }, SLIDER_COMMIT_TICKS),
        );
      });
      unsubscribes.push(() => row.volume.unsubscribe(volumeListener));
    }
    form.closeButton();

    // Fresh preferences from a feed update the visible rows in place; the
    // cache's syncing bracket keeps these writes from echoing as actions.
    const offPreferences = this.cache.onPreferences((prefs) => {
      for (const pref of prefs) {
        const row = rows.find((r) => r.target === pref.target);
        if (row) {
          row.heard.setData(!pref.muted);
          row.volume.setData(Math.round(pref.volume * 100));
        }
      }
    });

    await form.show();

    offPreferences();
    for (const unsubscribe of unsubscribes) {
      unsubscribe();
    }
  }
}
