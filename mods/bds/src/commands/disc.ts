import {
  system,
  Player,
  ItemStack,
  CommandPermissionLevel,
  CustomCommandParamType,
  CustomCommandStatus,
} from '@minecraft/server';
import type { CustomCommandOrigin, CustomCommandResult } from '@minecraft/server';

export class DiscCommand {
  static register(): void {
    system.beforeEvents.startup.subscribe((event) => {
      event.customCommandRegistry.registerCommand(
        {
          name: 'bvc:disk',
          description: 'Give yourself a BVC audio disc',
          cheatsRequired: false,
          permissionLevel: CommandPermissionLevel.GameDirectors,
          optionalParameters: [
            { name: 'audio_id', type: CustomCommandParamType.String },
          ],
        },
        (origin: CustomCommandOrigin, audioId?: string): CustomCommandResult | undefined => {
          const player = origin.sourceEntity;
          if (!player || player.typeId !== 'minecraft:player') {
            return {
              status: CustomCommandStatus.Failure,
              message: 'This command can only be run by a player',
            };
          }

          if (!audioId) {
            return {
              status: CustomCommandStatus.Failure,
              message: 'Usage: /bvc:disk <audio_id>',
            };
          }

          system.run(() => {
            DiscCommand.giveDisc(player as Player, audioId);
          });

          return { status: CustomCommandStatus.Success };
        }
      );
    });
  }

  private static giveDisc(player: Player, audioId: string): void {
    try {
      const inventory = player.getComponent('minecraft:inventory');
      if (!inventory || !inventory.container) {
        return;
      }

      const disc = new ItemStack('minecraft:music_disc_5', 1);
      disc.nameTag = `BVC: ${audioId}`;
      disc.setDynamicProperty('bvc:audio_id', audioId);
      disc.setDynamicProperty('bvc:is_bvc_disc', true);

      inventory.container.addItem(disc);
      player.sendMessage(`Gave you a BVC audio disc: ${audioId}`);
      console.info(`[BVC] Gave ${player.name} a BVC audio disc: ${audioId}`);
    } catch (e) {
      console.error('[BVC] Failed to create audio disc:', e);
    }
  }
}
