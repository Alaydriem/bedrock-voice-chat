import { system, world } from '@minecraft/server';
import type { Player } from '@minecraft/server';
import {
  CustomForm,
  ObservableBoolean,
  ObservableNumber,
  ObservableString,
} from '@minecraft/server-ui';
import { DataDrivenScreenClosedReason } from '@minecraft/server-ui';
import { JUKEBOX_TARGET, MAX_LEVEL } from '../control/jukebox';
import type { ControlSender } from '../control/sender';
import type { PlayerPreference, StateCache } from '../state/state_cache';
import { FormShow } from './form_show';
import type { PanelTestConfig } from './panel_test';

// Players within this many blocks are offered in the volumes list even if the
// owner has never adjusted them; already-adjusted players always appear.
export const NEARBY_RADIUS_BLOCKS = 64;

// Sliders fire on every notch; commit the value only after the drag settles.
const SLIDER_COMMIT_TICKS = 10;

// The hybrid main page shows inline controls for only this many nearest
// players; everyone else lives behind "All players…".
const QUICK_ROWS = 5;

// What the jukebox row is called on screen. The raw sentinel is a wire value, not a name.
const JUKEBOX_LABEL = 'Jukebox music';

// Dropdown presets for player rows. MUTED_VALUE encodes "Muted"; picking
// FINE_TUNE_VALUE navigates to the player's detail page instead of setting a
// level.
const MUTED_VALUE = -1;
const FINE_TUNE_VALUE = -2;
const PRESET_ITEMS = [
  { label: 'Muted', value: MUTED_VALUE },
  { label: '25%', value: 25 },
  { label: '50%', value: 50 },
  { label: '75%', value: 75 },
  { label: '100%', value: 100 },
  { label: 'Fine-tune…', value: FINE_TUNE_VALUE },
];

// Already-adjusted players with no live position sort after everyone nearby.
const FAR_AWAY = Number.POSITIVE_INFINITY;

interface VolumeTarget {
  target: string;
  distanceSq: number;
}

// The volumes view's page graph: main (nearest players inline) → list (every
// player, searchable) → detail (full controls for one player). 'exit' returns
// control to the panel; a detail page remembers which page it came from.
type VolumesPage =
  | { kind: 'main' }
  | { kind: 'list' }
  | { kind: 'detail'; target: string; back: 'main' | 'list' }
  | { kind: 'exit' };

/// The per-player volumes surface: side-by-side rows (name + state on the
/// left, a preset dropdown on the right) for the nearest few players, a
/// searchable all-players page with the same rows, and a fine-tune detail page
/// (hear toggle + exact slider) reachable from any row's dropdown. Rows are
/// sorted by distance and re-render live as preferences ride in from a feed.
/// All player-initiated changes go through the cache's pending shadows so
/// stale polls can't flap them back.
export class PlayerVolumesView {
  constructor(
    private readonly sender: ControlSender,
    private readonly cache: StateCache,
    // Rescopes the panel feed's preference fetches to what the current page
    // actually displays.
    private readonly setTargets: (targets: string[]) => void,
    private readonly panelTest: PanelTestConfig,
  ) {}

  // Runs the page loop until the player exits the volumes surface entirely.
  async show(owner: Player): Promise<void> {
    let page: VolumesPage = { kind: 'main' };
    while (page.kind !== 'exit') {
      switch (page.kind) {
        case 'main':
          page = await this.showMain(owner);
          break;
        case 'list':
          page = await this.showList(owner);
          break;
        case 'detail':
          page = await this.showDetail(owner, page.target, page.back);
          break;
      }
    }
  }

  // Proximity-scoped list: players within NEARBY_RADIUS_BLOCKS (same
  // dimension, excluding the owner) plus everyone the owner has already
  // adjusted, nearest first; synthetic test rows are appended when armed.
  private targets(owner: Player): VolumeTarget[] {
    const found = new Map<string, number>();
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
      const distanceSq = dx * dx + dy * dy + dz * dz;
      if (distanceSq <= NEARBY_RADIUS_BLOCKS * NEARBY_RADIUS_BLOCKS) {
        found.set(other.name, distanceSq);
      }
    }
    for (const adjusted of this.cache.adjustedTargets()) {
      // The jukebox is a pinned row of its own, not a player who happens to have a preference.
      // Folded in here it would render a second time, at the bottom, under its wire name.
      if (adjusted === JUKEBOX_TARGET) {
        continue;
      }
      if (!found.has(adjusted)) {
        found.set(adjusted, FAR_AWAY);
      }
    }
    for (let i = 1; i <= this.panelTest.count; i++) {
      const name = `Test${String(i).padStart(2, '0')}`;
      if (!found.has(name)) {
        found.set(name, (i * 8) ** 2);
      }
    }
    return [...found.entries()]
      .map(([target, distanceSq]) => ({ target, distanceSq }))
      .sort(
        (a, b) =>
          a.distanceSq - b.distanceSq || a.target.localeCompare(b.target),
      );
  }

  // What the state feed is asked to cover while a page is open: the players on it plus the
  // jukebox, whose row is pinned rather than listed. Omitting it leaves the pinned row unseeded —
  // the net poll scopes /api/preferences to this list and the no-net sync request carries it.
  private reportedTargets(targets: VolumeTarget[]): string[] {
    return [JUKEBOX_TARGET, ...targets.map((t) => t.target)];
  }

  private async showMain(owner: Player): Promise<VolumesPage> {
    const targets = this.targets(owner);
    this.setTargets(this.reportedTargets(targets));
    const quick = targets.slice(0, QUICK_ROWS);
    let next: VolumesPage = { kind: 'exit' };
    const unsubscribes: Array<() => void> = [];

    const form = new CustomForm(owner, 'Player volumes');

    // Pinned above the players and never sorted among them: the jukebox is always present, while
    // the rows below it come and go with who is nearby.
    form.label(JUKEBOX_LABEL);
    this.addPlayerRow(form, owner, JUKEBOX_TARGET, unsubscribes, undefined, () => {
      next = { kind: 'detail', target: JUKEBOX_TARGET, back: 'main' };
      form.close();
    });
    form.divider();

    if (targets.length === 0) {
      form.label('No players nearby');
    } else {
      form.label('Nearest');
    }
    for (const t of quick) {
      this.addPlayerRow(form, owner, t.target, unsubscribes, undefined, () => {
        next = { kind: 'detail', target: t.target, back: 'main' };
        form.close();
      });
    }
    if (targets.length > 0) {
      form.divider().button(`All players (${targets.length})…`, () => {
        next = { kind: 'list' };
        form.close();
      });
    }
    form.closeButton();

    const reason = await FormShow.withRetry(form);
    for (const unsubscribe of unsubscribes) {
      unsubscribe();
    }
    return reason === DataDrivenScreenClosedReason.ServerClosed
      ? next
      : { kind: 'exit' };
  }

  private async showList(owner: Player): Promise<VolumesPage> {
    const targets = this.targets(owner);
    this.setTargets(this.reportedTargets(targets));
    let next: VolumesPage = { kind: 'exit' };
    const unsubscribes: Array<() => void> = [];

    const form = new CustomForm(owner, 'All players');

    // Pinned above the search field rather than inside it. Hiding the jukebox behind a filter over
    // player names would lose the one row that is always there.
    const jukeboxLabel = new ObservableString(this.stateLabel(JUKEBOX_TARGET, false));
    form.button(jukeboxLabel, () => {
      next = { kind: 'detail', target: JUKEBOX_TARGET, back: 'list' };
      form.close();
    });
    unsubscribes.push(
      this.cache.onPreferences((prefs) => {
        if (prefs.some((p) => p.target === JUKEBOX_TARGET)) {
          jukeboxLabel.setData(this.stateLabel(JUKEBOX_TARGET, false));
        }
      }),
    );
    form.divider();

    if (targets.length === 0) {
      form.label('No players nearby');
    } else {
      const search = new ObservableString('', { clientWritable: true });
      form.textField('Search', search, {
        description: 'Type to filter players',
      });
      form.divider();

      const rowVisibility = new Map<string, ObservableBoolean>();
      for (const t of targets) {
        const visible = new ObservableBoolean(true);
        rowVisibility.set(t.target, visible);
        // Buttons are the toolkit's only one-line row, so the list stays a
        // compact scannable column; the row's full controls live behind it.
        const label = new ObservableString(this.stateLabel(t.target, false));
        form.button(
          label,
          () => {
            next = { kind: 'detail', target: t.target, back: 'list' };
            form.close();
          },
          { visible },
        );
        unsubscribes.push(
          this.cache.onPreferences((prefs) => {
            if (prefs.some((p) => p.target === t.target)) {
              label.setData(this.stateLabel(t.target, false));
            }
          }),
        );
      }

      const searchListener = search.subscribe((query) => {
        const needle = query.trim().toLowerCase();
        for (const [name, visible] of rowVisibility) {
          visible.setData(name.toLowerCase().includes(needle));
        }
      });
      unsubscribes.push(() => search.unsubscribe(searchListener));
    }
    form
      .divider()
      .button('← Back', () => {
        next = { kind: 'main' };
        form.close();
      })
      .closeButton();

    const reason = await FormShow.withRetry(form);
    for (const unsubscribe of unsubscribes) {
      unsubscribe();
    }
    return reason === DataDrivenScreenClosedReason.ServerClosed
      ? next
      : { kind: 'exit' };
  }

  private async showDetail(
    owner: Player,
    target: string,
    back: 'main' | 'list',
  ): Promise<VolumesPage> {
    this.setTargets([target]);
    const pref = this.cache.preferenceFor(target);
    // The detail page's label widget is the one surface that renders § color
    // codes, so only it gets the colored variant.
    const status = new ObservableString(this.stateLabel(target, true));
    const heard = new ObservableBoolean(!(pref?.muted ?? false), {
      clientWritable: true,
    });
    const volume = new ObservableNumber(
      Math.round((pref?.volume ?? 1) * 100),
      { clientWritable: true },
    );
    let next: VolumesPage = { kind: 'exit' };
    const unsubscribes: Array<() => void> = [];

    // Like the panel toggles: the syncing bracket covers synchronous setData
    // dispatch, the last-seen comparison covers asynchronous dispatch, so a
    // feed-applied update can never loop back into an action.
    let lastHeard = heard.getData();
    const heardListener = heard.subscribe((on) => {
      if (on === lastHeard) {
        return;
      }
      lastHeard = on;
      if (!this.cache.isSyncing) {
        this.cache.markHeardPending(target, on);
        void this.sender.send({ kind: 'hear', target, on }, owner);
        status.setData(this.stateLabel(target, true));
      }
    });
    unsubscribes.push(() => heard.unsubscribe(heardListener));

    let lastCommittedVolume = volume.getData();
    let pendingCommit: number | undefined;
    const volumeListener = volume.subscribe((value) => {
      if (this.cache.isSyncing) {
        // A feed apply that merely reflects this target's own unexpired shadow
        // must not become the commit baseline — the pending drag's value would
        // then compare equal and never be sent.
        if (!this.cache.hasPendingVolume(target)) {
          lastCommittedVolume = value;
        }
        return;
      }
      // Shadow every notch so a poll landing mid-drag can't yank the thumb.
      const clamped = Math.max(0, Math.min(MAX_LEVEL, Math.round(value)));
      this.cache.markVolumePending(target, clamped / 100);
      // Debounce the drag: only the value that survives the settle window is
      // committed.
      if (pendingCommit !== undefined) {
        system.clearRun(pendingCommit);
      }
      pendingCommit = system.runTimeout(() => {
        pendingCommit = undefined;
        if (value === lastCommittedVolume) {
          return;
        }
        lastCommittedVolume = value;
        void this.sender.send(
          { kind: 'volume', target, value: clamped },
          owner,
        );
        status.setData(this.stateLabel(target, true));
      }, SLIDER_COMMIT_TICKS);
    });
    unsubscribes.push(() => volume.unsubscribe(volumeListener));

    // Fresh preferences from a feed update the controls in place; the cache's
    // syncing bracket keeps these writes from echoing as actions.
    unsubscribes.push(
      this.cache.onPreferences((prefs) => {
        const p = prefs.find((x) => x.target === target);
        if (!p) {
          return;
        }
        heard.setData(!p.muted);
        volume.setData(Math.round(p.volume * 100));
        status.setData(this.stateLabel(target, true));
      }),
    );

    const form = new CustomForm(owner, this.displayName(target))
      .label(status)
      .divider()
      .toggle('Hear', heard)
      .slider('Volume', volume, 0, MAX_LEVEL, { step: 5 })
      .divider()
      .button('← Back to players', () => {
        next = { kind: back };
        form.close();
      })
      .closeButton();

    const reason = await FormShow.withRetry(form);
    for (const unsubscribe of unsubscribes) {
      unsubscribe();
    }
    return reason === DataDrivenScreenClosedReason.ServerClosed
      ? next
      : { kind: 'exit' };
  }

  // One quick row per player: the dropdown label is JUST the name — the
  // control box itself displays the current preset ("100%"/"Muted"), so
  // repeating state in the label would render it twice. Its "Fine-tune…" item
  // navigates to the detail page via `onFineTune`.
  private addPlayerRow(
    form: CustomForm,
    owner: Player,
    target: string,
    unsubscribes: Array<() => void>,
    visible: ObservableBoolean | undefined,
    onFineTune: () => void,
  ): void {
    const value = new ObservableNumber(
      this.presetFromPref(this.cache.preferenceFor(target)),
      { clientWritable: true },
    );
    form.dropdown(this.displayName(target), value, PRESET_ITEMS, visible ? { visible } : {});

    let lastSeen = value.getData();
    const listener = value.subscribe((v) => {
      if (v === lastSeen) {
        return;
      }
      lastSeen = v;
      if (this.cache.isSyncing) {
        return;
      }
      if (v === FINE_TUNE_VALUE) {
        onFineTune();
        return;
      }
      this.applyPreset(owner, target, v);
    });
    unsubscribes.push(() => value.unsubscribe(listener));

    unsubscribes.push(
      this.cache.onPreferences((prefs) => {
        const p = prefs.find((x) => x.target === target);
        if (!p) {
          return;
        }
        const preset = this.presetFromPref(p);
        if (preset !== value.getData()) {
          value.setData(preset);
        }
      }),
    );
  }

  private applyPreset(owner: Player, target: string, preset: number): void {
    const wasMuted = this.cache.preferenceFor(target)?.muted ?? false;
    if (preset === MUTED_VALUE) {
      this.cache.markHeardPending(target, false);
      void this.sender.send({ kind: 'hear', target, on: false }, owner);
      return;
    }
    if (wasMuted) {
      // Picking a level while muted implies wanting to hear them again.
      this.cache.markHeardPending(target, true);
      void this.sender.send({ kind: 'hear', target, on: true }, owner);
    }
    this.cache.markVolumePending(target, preset / 100);
    void this.sender.send(
      { kind: 'volume', target, value: preset },
      owner,
    );
  }

  // Player rows only offer presets, so the current volume snaps to the nearest
  // one for display; the detail page keeps the exact value.
  private presetFromPref(pref: PlayerPreference | undefined): number {
    if (pref?.muted) {
      return MUTED_VALUE;
    }
    const volume = Math.round((pref?.volume ?? 1) * 100);
    let best = 100;
    let bestDistance = Number.POSITIVE_INFINITY;
    for (const candidate of [25, 50, 75, 100]) {
      const distance = Math.abs(candidate - volume);
      if (distance < bestDistance) {
        bestDistance = distance;
        best = candidate;
      }
    }
    return best;
  }

  // "Name ●●●○ 70%" while heard, "Name ✕ muted" while not. § color codes only
  // render on label widgets (buttons and dropdown labels print them
  // literally), so they are opt-in via `colored`.
  private stateLabel(target: string, colored: boolean): string {
    const name = this.displayName(target);
    const pref = this.cache.preferenceFor(target);
    if (pref?.muted) {
      return colored ? `§8${name} ✕ muted§r` : `${name} ✕ muted`;
    }
    const volume = Math.round((pref?.volume ?? 1) * 100);
    const state = `${this.pips(volume)} ${volume}%`;
    return colored ? `§a${name}§r ${state}` : `${name} ${state}`;
  }

  // The jukebox rides the preference plane under a wire sentinel; every surface a player reads
  // shows its name instead.
  private displayName(target: string): string {
    return target === JUKEBOX_TARGET ? JUKEBOX_LABEL : target;
  }

  private pips(volume: number): string {
    const filled = Math.min(4, Math.max(0, Math.ceil(volume / 25)));
    return '●'.repeat(filled) + '○'.repeat(4 - filled);
  }
}
