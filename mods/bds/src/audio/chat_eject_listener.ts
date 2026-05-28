import { system, world } from '@minecraft/server';
import { Coordinates } from '../dto';
import type { AudioPlayerManager } from './player_manager';

const MAGIC = '!bvce ';

export class ChatEjectListener {
  constructor(
    private readonly audioManager: AudioPlayerManager,
    private readonly getWorldUuid: () => string,
  ) {}

  register(): void {
    world.beforeEvents.chatSend.subscribe((ev) => {
      if (!ev.message.startsWith(MAGIC)) return;
      ev.cancel = true;
      const parts = ev.message.slice(MAGIC.length).split(' ');
      if (parts.length !== 3) return;
      const [xs, ys, zs] = parts;
      const x = parseInt(xs, 10);
      const y = parseInt(ys, 10);
      const z = parseInt(zs, 10);
      if (!Number.isFinite(x) || !Number.isFinite(y) || !Number.isFinite(z)) return;
      const key = this.audioManager.locationKey(
        this.getWorldUuid(),
        new Coordinates(x, y, z),
      );
      system.run(() => {
        this.audioManager.forceEject(key);
      });
    });
  }
}
