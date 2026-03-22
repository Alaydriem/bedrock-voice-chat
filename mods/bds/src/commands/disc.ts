import {
  system,
  Player,
  ItemStack,
  CommandPermissionLevel,
  CustomCommandParamType,
  CustomCommandStatus,
} from '@minecraft/server';
import type { CustomCommandOrigin, CustomCommandResult } from '@minecraft/server';
import {
  http,
  HttpRequest,
  HttpRequestMethod,
  HttpHeader,
} from '@minecraft/server-net';

/**
 * Handles the /bvc:disk command.
 * Resolves an audio file name/ID via the server API, then gives the player a disc.
 */
export class DiscCommand {
  constructor(
    private readonly serverUrl: string,
    private readonly accessToken: string
  ) {}

  static register(serverUrl: string, accessToken: string): void {
    const cmd = new DiscCommand(serverUrl, accessToken);

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
            cmd.execute(player as Player, name);
          });

          return { status: CustomCommandStatus.Success };
        }
      );
    });
  }

  execute(player: Player, nameOrId: string): void {
    this.resolveAndGiveDisc(player, nameOrId);
  }

  private resolveAndGiveDisc(player: Player, nameOrId: string): void {
    const request = new HttpRequest(
      `${this.serverUrl}/api/audio/file`
    );
    request.setMethod((HttpRequestMethod as any).Get);
    request.setHeaders([
      new HttpHeader('X-MC-Access-Token', this.accessToken),
      new HttpHeader('Accept', 'application/json'),
    ]);
    request.setTimeout(3);

    http
      .request(request)
      .then((response) => {
        let audioId = nameOrId;

        if (response.status >= 200 && response.status < 300) {
          try {
            const files = JSON.parse(response.body) as Array<{
              id: string;
              original_filename: string;
            }>;
            const match =
              files.find((f) => f.id === nameOrId) ||
              files.find((f) =>
                f.original_filename
                  .toLowerCase()
                  .includes(nameOrId.toLowerCase())
              );
            if (match) {
              audioId = match.id;
            }
          } catch {
            // Use nameOrId as-is if parsing fails
          }
        }

        this.giveDisc(player, audioId, nameOrId);
      })
      .catch(() => {
        // Fallback: give disc with raw name/ID
        this.giveDisc(player, nameOrId, nameOrId);
      });
  }

  private giveDisc(
    player: Player,
    audioId: string,
    displayName: string
  ): void {
    try {
      const inventory = player.getComponent('minecraft:inventory');
      if (!inventory || !inventory.container) {
        return;
      }

      const disc = new ItemStack('minecraft:music_disc_5', 1);
      disc.nameTag = `BVC: ${displayName}`;
      disc.setDynamicProperty('bvc:audio_id', audioId);
      disc.setDynamicProperty('bvc:is_bvc_disc', true);

      inventory.container.addItem(disc);
      player.sendMessage(`Gave you a BVC audio disc: ${displayName}`);
      console.info(
        `[BVC] Gave ${player.name} a BVC audio disc: ${displayName} (id: ${audioId})`
      );
    } catch (e) {
      console.error('[BVC] Failed to create audio disc:', e);
    }
  }
}
