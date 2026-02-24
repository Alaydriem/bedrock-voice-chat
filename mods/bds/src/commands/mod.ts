import {
  system,
  CommandPermissionLevel,
  CustomCommandParamType,
  CustomCommandStatus,
} from '@minecraft/server';
import type { CustomCommandOrigin, CustomCommandResult, Player } from '@minecraft/server';
import { DiscCommand } from './disc';

/**
 * Register all BVC custom commands via the CustomCommandRegistry.
 */
export function registerCommands(
  serverUrl: string,
  accessToken: string
): void {
  const discCommand = new DiscCommand(serverUrl, accessToken);

  system.beforeEvents.startup.subscribe((event) => {
    event.customCommandRegistry.registerCommand(
      {
        name: 'bvc:disk',
        description: 'Give yourself a BVC audio disc',
        cheatsRequired: false,
        permissionLevel: CommandPermissionLevel.GameDirectors,
        optionalParameters: [
          { name: 'name', type: CustomCommandParamType.String },
        ],
      },
      (origin: CustomCommandOrigin, name?: string): CustomCommandResult | undefined => {
        const player = origin.sourceEntity;
        if (!player || player.typeId !== 'minecraft:player') {
          return {
            status: CustomCommandStatus.Failure,
            message: 'This command can only be run by a player',
          };
        }

        if (!name) {
          return {
            status: CustomCommandStatus.Failure,
            message: 'Usage: /bvc:disk <name_or_id>',
          };
        }

        system.run(() => {
          discCommand.execute(player as Player, name);
        });

        return { status: CustomCommandStatus.Success };
      }
    );
  });
}
