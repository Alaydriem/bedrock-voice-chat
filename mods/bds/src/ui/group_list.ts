import type { Player } from '@minecraft/server';
import { CustomForm, ObservableString } from '@minecraft/server-ui';
import type { ObservableBoolean } from '@minecraft/server-ui';
import { DataDrivenScreenClosedReason } from '@minecraft/server-ui';
import type { ControlSender } from '../control/sender';
import type { GroupRow } from '../state/group_cache';
import type { GroupSource } from '../state/group_source';
import type { StateCache } from '../state/state_cache';
import { FormShow } from './form_show';
import type { PanelTestConfig } from './panel_test';
import { SearchableRows } from './searchable_rows';

// Where the page goes when it closes. 'panel' returns to the control panel;
// 'refresh' rebuilds this page, which is the only way to change a form's rows.
type GroupsExit = 'panel' | 'refresh';

/// The group surface: every group on the server as a one-line row that joins on
/// tap, over a search field, plus create, the copyable share code, and leave.
export class GroupListView {
  constructor(
    private readonly sender: ControlSender,
    private readonly source: GroupSource,
    private readonly cache: StateCache,
    private readonly panelTest: PanelTestConfig,
    // Leave is gated on known group membership only when the feed actually
    // tracks it (net). No-net cannot know the group, so Leave stays clickable
    // there and a stray leave is a harmless server-side no-op.
    private readonly leaveDisabled: ObservableBoolean | boolean,
  ) {}

  /// Shows the page until it closes. Resolves true when the panel should reopen.
  async show(player: Player): Promise<boolean> {
    for (;;) {
      const exit = await this.page(player);
      if (exit !== 'refresh') {
        return exit === 'panel';
      }
    }
  }

  private async page(player: Player): Promise<GroupsExit | null> {
    // Resolved before the form is built: a row cannot be added to a form that is
    // already on screen.
    const rows = await this.rows(player);
    if (!player.isValid) {
      return null;
    }

    let exit: GroupsExit = 'panel';
    const unsubscribes: Array<() => void> = [];
    const form = new CustomForm(player, 'Groups');
    const current = this.cache.group;

    if (rows === null) {
      form.label('Group list unavailable — press Refresh');
      form.divider();
    } else if (rows.length === 0) {
      form.label('No groups yet — press Create group');
      form.divider();
    } else {
      const searchable = rows.map((row) => ({
        key: row.name,
        // The current group is marked and inert: joining it again is a no-op the
        // server already absorbs, and a row that does nothing should say why.
        // A button renders § codes literally, so the marker is plain text.
        label: new ObservableString(
          row.id === current ? `* ${row.name} (current)` : row.name,
        ),
        onSelect: (): void => {
          if (row.id === current) {
            return;
          }
          void this.join(player, row);
          form.close();
        },
      }));
      unsubscribes.push(
        ...SearchableRows.attach(form, searchable, {
          placeholder: 'Search',
          description: 'Type to filter groups',
        }),
      );
      form.divider();
    }

    // Seeded with the current group's code: the text field is the only DDUI widget
    // whose text a player can select and copy, so it doubles as the share surface.
    const joinCode = new ObservableString(current ?? '', {
      clientWritable: true,
    });

    form
      .button('Create group', () => {
        void this.create(player, joinCode);
        form.close();
      })
      .textField('Group code', joinCode, {
        description: current
          ? 'Your current group code — copy it to share, or paste another and press Join by code'
          : 'Paste a share code, then press Join by code',
      })
      .button('Join by code', () => {
        const code = joinCode.getData().trim();
        if (code.length > 0) {
          void this.sender.send({ kind: 'group-join', channel: code }, player);
        }
        form.close();
      })
      .button(
        'Leave group',
        () => {
          void this.sender.send({ kind: 'group-leave' }, player);
          form.close();
        },
        { disabled: this.leaveDisabled },
      )
      .divider()
      .button('Refresh', () => {
        exit = 'refresh';
        form.close();
      })
      .button('← Back', () => {
        exit = 'panel';
        form.close();
      })
      .closeButton();

    const reason = await FormShow.withRetry(form);
    for (const unsubscribe of unsubscribes) {
      unsubscribe();
    }
    return reason === DataDrivenScreenClosedReason.ServerClosed ? exit : null;
  }

  private async rows(player: Player): Promise<GroupRow[] | null> {
    const rows = await this.source.list(player);
    if (this.panelTest.count === 0) {
      return rows;
    }
    const synthetic: GroupRow[] = [];
    for (let i = 0; i < this.panelTest.count; i++) {
      synthetic.push({ id: `paneltest-${i}`, name: `Test Group ${i + 1}` });
    }
    // Synthetic rows exist to exercise the layout, so they show even when the real
    // list could not be read.
    return [...(rows ?? []), ...synthetic];
  }

  private async join(player: Player, row: GroupRow): Promise<void> {
    const result = await this.sender.send(
      { kind: 'group-join', channel: row.id },
      player,
    );
    if (!result.ok) {
      player.sendMessage('§c[BVC] Group join failed; try again');
      return;
    }
    player.sendMessage(`§a[BVC] Joined §f${row.name}`);
  }

  private async create(
    player: Player,
    joinCode: ObservableString,
  ): Promise<void> {
    const result = await this.sender.send({ kind: 'group-create' }, player);
    if (!result.ok) {
      player.sendMessage('§c[BVC] Group create failed; try again');
      return;
    }
    if (result.groupCode) {
      joinCode.setData(result.groupCode);
      player.sendMessage(
        `§a[BVC] Group created — share code: §f${result.groupCode}`,
      );
    }
  }
}
