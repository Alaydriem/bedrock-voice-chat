import { system } from '@minecraft/server';
import type { Player } from '@minecraft/server';
import { CustomForm, MessageBox, Observable } from '@minecraft/server-ui';
import { DataDrivenScreenClosedReason } from '@minecraft/server-ui';
import type { ControlSender } from '../control/sender';
import type { StateCacheStore } from '../state/cache_store';
import type { PanelFeed } from '../state/state_source';
import { PlayerVolumesView } from './player_list';

// The chat screen the /bvc command was typed into may still be closing when the
// form first shows; retry a few times before giving up.
const USER_BUSY_RETRIES = 5;
const USER_BUSY_RETRY_TICKS = 10;
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
    const volumes = new PlayerVolumesView(sender, cache);

    // The feed's preference fetches stay scoped to what is actually displayed:
    // the volumes view swaps in its row targets while it is open.
    let displayedTargets: string[] = cache.adjustedTargets();
    const feed = this.getFeed();
    feed?.start(player, cache, () => displayedTargets);

    const syncTimeout = system.runTimeout(() => {
      if (!cache.synced) {
        cache.status.setData('State unavailable');
      }
    }, SYNC_TIMEOUT_TICKS);

    const unsubscribes: Array<() => void> = [];
    const subscribe = <T extends string | number | boolean>(
      observable: Observable<T>,
      onPlayerChange: (value: T) => void,
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
      void sender.send({ kind: 'mute', on }, player);
    });
    subscribe(cache.deafened, (on) => {
      void sender.send({ kind: 'deafen', on }, player);
    });

    const recordLabel = Observable.create<string>(
      cache.recording.getData() ? 'Stop recording' : 'Start recording',
    );
    const recordListener = cache.recording.subscribe((recording) => {
      recordLabel.setData(recording ? 'Stop recording' : 'Start recording');
    });
    unsubscribes.push(() => cache.recording.unsubscribe(recordListener));

    const joinCode = Observable.create<string>('', { clientWritable: true });

    let reopenAfterVolumes = false;
    const form = CustomForm.create(player, 'BVC Voice Controls')
      .label(cache.status)
      .divider()
      .toggle('Muted', cache.muted, { disabled: cache.controlsLocked })
      .toggle('Deafened', cache.deafened, { disabled: cache.controlsLocked })
      .button(
        recordLabel,
        () => {
          void this.onRecordPressed(player, sender, cache.recording.getData());
        },
        { disabled: cache.controlsLocked },
      )
      .divider()
      .button('Create group', () => {
        void this.onCreateGroup(player, sender, cache.status);
      })
      .textField('Group code', joinCode, {
        description: 'Paste a share code, then press Join',
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

    const closedReason = await this.showWithRetry(form);

    if (
      reopenAfterVolumes &&
      closedReason === DataDrivenScreenClosedReason.ServerClose
    ) {
      const rows = volumes.rows(player);
      displayedTargets = rows.map((r) => r.target);
      feed?.start(player, cache, () => displayedTargets);
      await volumes.show(player, rows);
    }

    system.clearRun(syncTimeout);
    for (const unsubscribe of unsubscribes) {
      unsubscribe();
    }
    feed?.stop(player.name);

    if (reopenAfterVolumes) {
      // Back out of the volumes view into a fresh panel.
      this.open(player);
    }
  }

  private async onRecordPressed(
    player: Player,
    sender: ControlSender,
    recording: boolean,
  ): Promise<void> {
    if (recording) {
      await sender.send({ kind: 'record', on: false }, player);
      return;
    }
    const result = await MessageBox.create(player, 'Start recording?')
      .body('This records your voice session on your desktop app.')
      .button1('Start')
      .button2('Cancel')
      .show();
    if (result.selection === 0) {
      await sender.send({ kind: 'record', on: true }, player);
    }
  }

  private async onCreateGroup(
    player: Player,
    sender: ControlSender,
    status: Observable<string>,
  ): Promise<void> {
    const result = await sender.send({ kind: 'group-create' }, player);
    if (!result.ok) {
      player.sendMessage('§c[BVC] Group create failed; try again');
      return;
    }
    if (result.groupCode) {
      status.setData(`Group: ${result.groupCode}`);
      player.sendMessage(
        `§a[BVC] Group created — share code: §f${result.groupCode}`,
      );
    }
  }

  private async showWithRetry(
    form: CustomForm,
  ): Promise<DataDrivenScreenClosedReason> {
    let reason = await form.show();
    for (
      let attempt = 0;
      reason === DataDrivenScreenClosedReason.UserBusy &&
      attempt < USER_BUSY_RETRIES;
      attempt++
    ) {
      await this.wait(USER_BUSY_RETRY_TICKS);
      reason = await form.show();
    }
    return reason;
  }

  private wait(ticks: number): Promise<void> {
    return new Promise((resolve) => system.runTimeout(resolve, ticks));
  }
}
