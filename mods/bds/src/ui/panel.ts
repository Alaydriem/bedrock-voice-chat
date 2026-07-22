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
import type { StateCache } from '../state/state_cache';
import type { PanelFeed } from '../state/state_source';
import { FormShow } from './form_show';
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
  ) {}

  open(player: Player): void {
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

    // Seeded with the current group's code: the text field is the only DDUI
    // widget whose text a player can select and copy, so it doubles as the
    // "share my group" surface.
    const joinCode = new ObservableString(cache.group ?? '', {
      clientWritable: true,
    });
    // The seed above races the first snapshot (cache.group may still be null on
    // the first open); backfill the field once state rides in, but never clobber
    // something the player typed.
    const codeBackfill = cache.status.subscribe(() => {
      const group = cache.group;
      if (group && joinCode.getData() === '') {
        joinCode.setData(group);
      }
    });
    unsubscribes.push(() => cache.status.unsubscribe(codeBackfill));

    let reopenAfterVolumes = false;
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
      .button('Create group', () => {
        void this.onCreateGroup(player, sender, cache.status, joinCode);
      })
      .textField('Group code', joinCode, {
        description: cache.group
          ? 'Your current group code — copy it to share, or paste another and press Join'
          : 'Paste a share code, then press Join',
      })
      .button('Join group', () => {
        const code = joinCode.getData().trim();
        if (code.length > 0) {
          void sender.send({ kind: 'group-join', channel: code }, player);
        }
      })
      .button(
        'Leave group',
        () => {
          void sender.send({ kind: 'group-leave' }, player);
        },
        { disabled: cache.noGroup },
      )
      .divider()
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

    system.clearRun(syncTimeout);
    for (const unsubscribe of unsubscribes) {
      unsubscribe();
    }
    feed?.stop(player.name);

    if (closedReason === DataDrivenScreenClosedReason.ServerClosed) {
      if (confirmRecordAfterClose) {
        await this.confirmStartRecording(player, sender, cache);
      }
      if (reopenAfterVolumes || confirmRecordAfterClose) {
        // Back out of the volumes view / record confirmation into a fresh panel.
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

  private async onCreateGroup(
    player: Player,
    sender: ControlSender,
    status: ObservableString,
    joinCode: ObservableString,
  ): Promise<void> {
    const result = await sender.send({ kind: 'group-create' }, player);
    if (!result.ok) {
      player.sendMessage('§c[BVC] Group create failed; try again');
      return;
    }
    if (result.groupCode) {
      status.setData(`Group: ${result.groupCode}`);
      // Land the code in the copyable text field, not just the status label.
      joinCode.setData(result.groupCode);
      player.sendMessage(
        `§a[BVC] Group created — share code: §f${result.groupCode}`,
      );
    }
  }

}
