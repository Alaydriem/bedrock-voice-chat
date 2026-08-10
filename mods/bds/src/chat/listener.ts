import { world } from '@minecraft/server';
import type {
  ChatSendAfterEvent,
  EntityDieAfterEvent,
  Player,
  PlayerJoinAfterEvent,
  PlayerLeaveAfterEvent,
} from '@minecraft/server';
import type { ChatChannel } from './channel';
import { DeathMessage } from './death_message';

/**
 * Bridges in-game chat and the BVC chat channel.
 *
 * Four sources rather than one. Player chat arrives on `chatSend`; deaths, joins and leaves
 * reach players as messages the server composes, and the scripting API exposes each as its own
 * event. Subscribing to chat alone is why a BDS world showed conversation and nothing else.
 *
 * All four are ordinary after-events — none of the restricted-execution workarounds
 * `BvcsListener` needs for the before-event apply here.
 *
 * Console `/say` is absent from this list because BDS exposes no event for it: there is no
 * command hook, and `afterEvents.messageReceive` is documented as non-functional. Paper reads
 * it through `ServerCommandEvent`, and Bedrock has no counterpart.
 */
export class ChatListener {
  constructor(private readonly channel: ChatChannel) {}

  register(): void {
    world.afterEvents.chatSend.subscribe((ev: ChatSendAfterEvent) => {
      // BVC's own silent rides ride on chat. They are cancelled before players see them, but
      // the after-event still fires for some, and relaying one would put a player's audio
      // state into the app's chat log.
      if (ChatListener.isRide(ev.message)) {
        return;
      }
      this.channel.report(ev.sender.name, ev.message);
    });

    world.afterEvents.entityDie.subscribe(
      (ev: EntityDieAfterEvent) => {
        // A world with death messages switched off showed nobody this line in game, and
        // relaying it would put in the app exactly what the operator suppressed.
        if (!world.gameRules.showDeathMessages) {
          return;
        }
        // The subscription filter guarantees a player, whose `name` is the gamertag. `nameTag`
        // is not the same thing: it is freely rewritable, so a player can word another
        // player's death line by renaming themselves.
        const victim = ev.deadEntity as Player;
        this.channel.event(DeathMessage.render(victim.name, ev.damageSource));
      },
      { entityTypes: ['minecraft:player'] },
    );

    world.afterEvents.playerJoin.subscribe((ev: PlayerJoinAfterEvent) => {
      this.channel.event(`${ev.playerName} joined the game`);
    });

    world.afterEvents.playerLeave.subscribe((ev: PlayerLeaveAfterEvent) => {
      this.channel.event(`${ev.playerName} left the game`);
    });
  }

  /**
   * Broadcasts a line composed in the app, formatted to match vanilla so it is
   * indistinguishable from something typed in game.
   *
   * `world.sendMessage` does not fire `chatSend`, so this is never reported back and there is
   * nothing to suppress.
   */
  say(author: string, text: string): void {
    world.sendMessage(`<${author}> ${text}`);
  }

  private static isRide(message: string): boolean {
    return (
      message.startsWith('!bvcp ') ||
      message.startsWith('!bvca ') ||
      message.startsWith('!bvce ') ||
      message.startsWith('!bvcs:')
    );
  }
}
