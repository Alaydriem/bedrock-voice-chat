import { system } from '@minecraft/server';
import type { Player } from '@minecraft/server';
import {
  CustomForm,
  ObservableBoolean,
  ObservableString,
} from '@minecraft/server-ui';
import { DataDrivenScreenClosedReason } from '@minecraft/server-ui';
import type { ControlSender } from '../control/sender';
import type { StateCacheStore } from '../state/cache_store';
import type { GroupSource } from '../state/group_source';
import type { StateCache } from '../state/state_cache';
import type { PanelFeed } from '../state/state_source';
import { FormShow } from './form_show';
import { GroupListView } from './group_list';
import type { PanelTestConfig } from './panel_test';
import { PlayerVolumesView } from './player_list';

// If no snapshot lands in this window the panel stops claiming "Syncing…".
const SYNC_TIMEOUT_TICKS = 40;

/// The in-game voice control panel (`/bvc:panel`): a DDUI CustomForm whose
/// toggles and status line bind to the player's `StateCache` Observables, so an
/// open panel re-renders live as state rides in (net poll or !bvcs:). Controls
/// stay disabled until the first snapshot confirms real state — the panel never
/// shows an actionable, unconfirmed on/off.
export class ControlPanel {
  constructor(
    private readonly getSender: () => ControlSender | null,
    private readonly cacheStore: StateCacheStore,
    private readonly getFeed: () => PanelFeed | null,
    private readonly panelTest: PanelTestConfig,
    private readonly getGroupSource: () => GroupSource | null,
  ) {}

  open(player: Player): void {
    // The reopen paths below re-enter after awaits (volumes view, record
    // confirmation); a player who disconnected in that window is invalid and
    // reading player.name would throw. Bail before touching it.
    if (!player.isValid) {
      return;
    }
    const sender = this.getSender();
    if (!sender) {
      player.sendMessage('§c[BVC] Still starting up; try again in a moment');
      return;
    }
    void this.session(player, sender);
  }

  private async session(player: Player, sender: ControlSender): Promise<void> {
    const cache = this.cacheStore.for(player.name);

    // The feed's preference fetches stay scoped to what is actually displayed:
    // the volumes view swaps in its page's targets while it is open.
    let displayedTargets: string[] = cache.adjustedTargets();
    const feed = this.getFeed();
    const volumes = new PlayerVolumesView(
      sender,
      cache,
      (targets) => {
        displayedTargets = targets;
        feed?.start(player, cache, () => displayedTargets);
      },
      this.panelTest,
    );
    feed?.start(player, cache, () => displayedTargets);
    // The Groups page must resolve its rows before it builds its form, so ask
    // early. Net mode's warm is a no-op; only the no-net ride needs a head start.
    this.getGroupSource()?.warm(player);

    const syncTimeout = system.runTimeout(() => {
      if (!cache.synced) {
        cache.status.setData('State unavailable');
      }
    }, SYNC_TIMEOUT_TICKS);

    const unsubscribes: Array<() => void> = [];
    const subscribe = (
      observable: ObservableBoolean,
      onPlayerChange: (value: boolean) => void,
    ): void => {
      // Two independent echo guards, because the beta API does not document
      // subscriber-dispatch timing: the syncing bracket suppresses echoes when
      // setData notifies synchronously, and the last-seen comparison breaks any
      // feed→action→report→feed loop when it notifies asynchronously (a repeat
      // of a value already acted on is never re-sent).
      let lastSeen = observable.getData();
      const listener = observable.subscribe((value) => {
        if (value === lastSeen) {
          return;
        }
        lastSeen = value;
        if (!cache.isSyncing) {
          onPlayerChange(value);
        }
      });
      unsubscribes.push(() => observable.unsubscribe(listener));
    };

    subscribe(cache.muted, (on) => {
      cache.markPending('muted', on);
      void sender.send({ kind: 'mute', on }, player);
    });
    subscribe(cache.deafened, (on) => {
      cache.markPending('deafened', on);
      void sender.send({ kind: 'deafen', on }, player);
    });

    const recordLabel = new ObservableString(
      cache.recording.getData() ? 'Stop recording' : 'Start recording',
    );
    const recordListener = cache.recording.subscribe((recording) => {
      recordLabel.setData(recording ? 'Stop recording' : 'Start recording');
    });
    unsubscribes.push(() => cache.recording.unsubscribe(recordListener));

    let reopenAfterVolumes = false;
    let reopenAfterGroups = false;
    // A MessageBox cannot show over an open form (UserBusy, no selection), so
    // the record confirmation runs after this panel closes.
    let confirmRecordAfterClose = false;
    const form = new CustomForm(player, 'BVC Voice Controls')
      .label(cache.status)
      .divider()
      .toggle('Muted', cache.muted, { disabled: cache.controlsLocked })
      .toggle('Deafened', cache.deafened, { disabled: cache.controlsLocked })
      .button(
        recordLabel,
        () => {
          if (cache.recording.getData()) {
            cache.markPending('recording', false);
            void sender.send({ kind: 'record', on: false }, player);
            return;
          }
          confirmRecordAfterClose = true;
          form.close();
        },
        { disabled: cache.controlsLocked },
      )
      .divider()
      .button('Groups…', () => {
        reopenAfterGroups = true;
        form.close();
      })
      .button('Player volumes…', () => {
        reopenAfterVolumes = true;
        form.close();
      })
      .closeButton();

    const closedReason = await FormShow.withRetry(form);

    if (
      reopenAfterVolumes &&
      closedReason === DataDrivenScreenClosedReason.ServerClosed
    ) {
      await volumes.show(player);
    }

    if (
      reopenAfterGroups &&
      closedReason === DataDrivenScreenClosedReason.ServerClosed
    ) {
      const source = this.getGroupSource();
      if (source) {
        // Leave is gated on known group membership only when the feed actually
        // tracks it (net). No-net cannot know the group, so keep Leave always
        // clickable there and let a stray leave be a harmless server-side no-op.
        const leaveDisabled: ObservableBoolean | boolean =
          (feed?.tracksCurrentGroup() ?? true) ? cache.noGroup : false;
        await new GroupListView(
          sender,
          source,
          cache,
          this.panelTest,
          leaveDisabled,
        ).show(player);
      } else {
        player.sendMessage('§c[BVC] Still starting up; try again in a moment');
      }
    }

    system.clearRun(syncTimeout);
    for (const unsubscribe of unsubscribes) {
      unsubscribe();
    }
    feed?.stop(player.name);

    if (closedReason === DataDrivenScreenClosedReason.ServerClosed) {
      if (confirmRecordAfterClose) {
        await this.confirmStartRecording(player, sender, cache);
      }
      if (
        (reopenAfterVolumes || reopenAfterGroups || confirmRecordAfterClose) &&
        player.isValid
      ) {
        // Back out of a sub-view / the record confirmation into a fresh panel.
        this.open(player);
      }
    }
  }

  private async confirmStartRecording(
    player: Player,
    sender: ControlSender,
    cache: StateCache,
  ): Promise<void> {
    // The confirmation is a CustomForm whose Start button fires a server-side
    // callback — the same mechanism as the panel's own buttons — so confirming
    // never depends on reading a selection index out of a close result.
    let confirmed = false;
    const form = new CustomForm(player, 'Start recording?')
      .label('This records your voice session on your desktop app.')
      .button('Start', () => {
        confirmed = true;
        form.close();
      })
      .closeButton();
    const reason = await FormShow.withRetry(form);
    if (reason === DataDrivenScreenClosedReason.UserBusy) {
      player.sendMessage(
        '§c[BVC] Could not open the recording confirmation; try again',
      );
      return;
    }
    if (!confirmed) {
      return;
    }
    cache.markPending('recording', true);
    const result = await sender.send({ kind: 'record', on: true }, player);
    if (!result.ok) {
      cache.markPending('recording', false);
      player.sendMessage('§c[BVC] Recording failed to start; try again');
    }
  }
}
