import { world } from '@minecraft/server';
import type { ChatSendBeforeEvent } from '@minecraft/server';

const MAGIC_PRESENCE = '!bvcp ';
const MAGIC_ANNOUNCE = '!bvca ';

export class ChatPresenceListener {
  buildHandler(): (ev: ChatSendBeforeEvent) => void {
    return (ev: ChatSendBeforeEvent): void => {
      if (
        !ev.message.startsWith(MAGIC_PRESENCE) &&
        !ev.message.startsWith(MAGIC_ANNOUNCE)
      ) {
        return;
      }

      ev.cancel = true;
    };
  }

  register(): void {
    world.beforeEvents.chatSend.subscribe(this.buildHandler());
  }
}
