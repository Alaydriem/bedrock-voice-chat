import { world } from '@minecraft/server';
import type { ChatSendBeforeEvent } from '@minecraft/server';

const MAGIC = '!bvcp ';

export class ChatPresenceListener {
  buildHandler(): (ev: ChatSendBeforeEvent) => void {
    return (ev: ChatSendBeforeEvent): void => {
      if (!ev.message.startsWith(MAGIC)) {
        return;
      }

      ev.cancel = true;
    };
  }

  register(): void {
    world.beforeEvents.chatSend.subscribe(this.buildHandler());
  }
}
