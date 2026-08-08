import { world } from '@minecraft/server';
import type { ChatSendAfterEvent } from '@minecraft/server';
import type { ChatChannel } from './channel';

/**
 * Bridges in-game chat and the BVC chat channel.
 *
 * Reads through `afterEvents.chatSend`, which is an ordinary after-event — none of the
 * restricted-execution workarounds `BvcsListener` needs for the before-event apply here.
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
