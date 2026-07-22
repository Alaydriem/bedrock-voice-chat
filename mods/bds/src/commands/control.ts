import {
  system,
  world,
  Player,
  CommandPermissionLevel,
  CustomCommandParamType,
  CustomCommandStatus,
} from '@minecraft/server';
import type {
  CustomCommandOrigin,
  CustomCommandResult,
} from '@minecraft/server';
import type { ControlAction } from '../control/action';
import type { ControlSender } from '../control/sender';
import type { PanelTestConfig } from '../ui/panel_test';

export class ControlCommands {
  // The sender is resolved asynchronously (after config load), but commands must be
  // registered during startup — so the sender is provided lazily and read at
  // invocation time. `openPanel` opens the DDUI control panel for bare /bvc:panel.
  static register(
    getSender: () => ControlSender | null,
    openPanel: (player: Player) => void,
    panelTest: PanelTestConfig,
  ): void {
    system.beforeEvents.startup.subscribe((event) => {
      const registry = event.customCommandRegistry;

      const dispatch = (
        origin: CustomCommandOrigin,
        action: ControlAction,
      ): CustomCommandResult => {
        const player = origin.sourceEntity;
        if (!player || player.typeId !== 'minecraft:player') {
          return {
            status: CustomCommandStatus.Failure,
            message: 'This command can only be run by a player',
          };
        }
        const sender = getSender();
        if (!sender) {
          return {
            status: CustomCommandStatus.Failure,
            message: 'BVC is still starting up; try again in a moment',
          };
        }
        const acting = player as Player;
        system.run(() => {
          void (async () => {
            const result = await sender.send(action, acting);
            // The share code is the whole point of group-create; surface it (or
            // the failure) to the player instead of discarding the reply.
            if (action.kind === 'group-create') {
              if (result.groupCode) {
                acting.sendMessage(
                  `§a[BVC] Group created — share code: §f${result.groupCode}`,
                );
              } else if (!result.ok) {
                acting.sendMessage('§c[BVC] Group create failed; try again');
              }
            } else if (!result.ok) {
              acting.sendMessage('§c[BVC] Action failed; try again');
            }
          })();
        });
        return { status: CustomCommandStatus.Success };
      };

      registry.registerCommand(
        {
          name: 'bvc:panel',
          description: 'Open the BVC voice control panel',
          cheatsRequired: false,
          permissionLevel: CommandPermissionLevel.Any,
        },
        (origin: CustomCommandOrigin) => {
          const player = origin.sourceEntity;
          if (!player || player.typeId !== 'minecraft:player') {
            return {
              status: CustomCommandStatus.Failure,
              message: 'This command can only be run by a player',
            };
          }
          const acting = player as Player;
          system.run(() => openPanel(acting));
          return { status: CustomCommandStatus.Success };
        },
      );

      const boolParam = (name: string) => ({
        name,
        type: CustomCommandParamType.Boolean,
      });
      const strParam = (name: string) => ({
        name,
        type: CustomCommandParamType.String,
      });
      const intParam = (name: string) => ({
        name,
        type: CustomCommandParamType.Integer,
      });

      registry.registerCommand(
        {
          name: 'bvc:mute',
          description: 'Mute or unmute your microphone',
          cheatsRequired: false,
          permissionLevel: CommandPermissionLevel.Any,
          mandatoryParameters: [boolParam('on')],
        },
        (origin: CustomCommandOrigin, on: boolean) =>
          dispatch(origin, { kind: 'mute', on }),
      );

      registry.registerCommand(
        {
          name: 'bvc:deafen',
          description: 'Deafen or undeafen (mute everyone)',
          cheatsRequired: false,
          permissionLevel: CommandPermissionLevel.Any,
          mandatoryParameters: [boolParam('on')],
        },
        (origin: CustomCommandOrigin, on: boolean) =>
          dispatch(origin, { kind: 'deafen', on }),
      );

      registry.registerCommand(
        {
          name: 'bvc:record',
          description: 'Start or stop recording your session',
          cheatsRequired: false,
          permissionLevel: CommandPermissionLevel.Any,
          mandatoryParameters: [boolParam('on')],
        },
        (origin: CustomCommandOrigin, on: boolean) =>
          dispatch(origin, { kind: 'record', on }),
      );

      registry.registerCommand(
        {
          name: 'bvc:volume',
          description: 'Set your local volume for another player (0-100)',
          cheatsRequired: false,
          permissionLevel: CommandPermissionLevel.Any,
          mandatoryParameters: [strParam('player'), intParam('level')],
        },
        (origin: CustomCommandOrigin, player: string, level: number) => {
          const target = ControlCommands.resolveTarget(player);
          if (!target) {
            return ControlCommands.unknownPlayer(player);
          }
          return dispatch(origin, {
            kind: 'volume',
            target,
            value: Math.max(0, Math.min(100, level)),
          });
        },
      );

      registry.registerCommand(
        {
          name: 'bvc:hear',
          description: 'Choose whether you hear another player',
          cheatsRequired: false,
          permissionLevel: CommandPermissionLevel.Any,
          mandatoryParameters: [strParam('player'), boolParam('on')],
        },
        (origin: CustomCommandOrigin, player: string, on: boolean) => {
          const target = ControlCommands.resolveTarget(player);
          if (!target) {
            return ControlCommands.unknownPlayer(player);
          }
          return dispatch(origin, { kind: 'hear', target, on });
        },
      );

      registry.registerCommand(
        {
          name: 'bvc:groupcreate',
          description: 'Create a group and get its share code',
          cheatsRequired: false,
          permissionLevel: CommandPermissionLevel.Any,
        },
        (origin: CustomCommandOrigin) =>
          dispatch(origin, { kind: 'group-create' }),
      );

      registry.registerCommand(
        {
          name: 'bvc:groupjoin',
          description: 'Join a group by its share code',
          cheatsRequired: false,
          permissionLevel: CommandPermissionLevel.Any,
          mandatoryParameters: [strParam('code')],
        },
        (origin: CustomCommandOrigin, code: string) =>
          dispatch(origin, { kind: 'group-join', channel: code }),
      );

      registry.registerCommand(
        {
          name: 'bvc:groupleave',
          description: 'Leave your current group',
          cheatsRequired: false,
          permissionLevel: CommandPermissionLevel.Any,
        },
        (origin: CustomCommandOrigin) =>
          dispatch(origin, { kind: 'group-leave' }),
      );

      // Operator-only layout testing: pads the volumes view with synthetic
      // rows so it can be exercised without that many real nearby players.
      registry.registerCommand(
        {
          name: 'bvc:paneltest',
          description:
            'Pad the volumes view with synthetic players (0 to clear)',
          cheatsRequired: false,
          permissionLevel: CommandPermissionLevel.GameDirectors,
          mandatoryParameters: [intParam('count')],
        },
        (_origin: CustomCommandOrigin, count: number) => {
          panelTest.set(count);
          return {
            status: CustomCommandStatus.Success,
            message: `Volumes view padded with ${panelTest.count} synthetic players`,
          };
        },
      );
    });
  }

  // Per-player preferences key on the target's EXACT gamertag everywhere
  // downstream (the desktop gain store, the sink's name remap, the player
  // cards) — a typed name must resolve to a real online player's exact name,
  // never pass through raw. This is what makes the CLI land on the same path
  // the DDUI volumes view uses (whose rows are always exact names).
  private static resolveTarget(typed: string): string | null {
    const lowered = typed.toLowerCase();
    for (const p of world.getAllPlayers()) {
      if (p.name.toLowerCase() === lowered) {
        return p.name;
      }
    }
    return null;
  }

  private static unknownPlayer(typed: string): CustomCommandResult {
    return {
      status: CustomCommandStatus.Failure,
      message: `Unknown player: ${typed}`,
    };
  }
}
