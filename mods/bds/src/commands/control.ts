import {
  system,
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

export class ControlCommands {
  // The sender is resolved asynchronously (after config load), but commands must be
  // registered during startup — so the sender is provided lazily and read at
  // invocation time.
  static register(getSender: () => ControlSender | null): void {
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
          void sender.send(action, acting);
        });
        return { status: CustomCommandStatus.Success };
      };

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
        (origin: CustomCommandOrigin, player: string, level: number) =>
          dispatch(origin, {
            kind: 'volume',
            target: player,
            value: Math.max(0, Math.min(100, level)),
          }),
      );

      registry.registerCommand(
        {
          name: 'bvc:hear',
          description: 'Choose whether you hear another player',
          cheatsRequired: false,
          permissionLevel: CommandPermissionLevel.Any,
          mandatoryParameters: [strParam('player'), boolParam('on')],
        },
        (origin: CustomCommandOrigin, player: string, on: boolean) =>
          dispatch(origin, { kind: 'hear', target: player, on }),
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
    });
  }
}
