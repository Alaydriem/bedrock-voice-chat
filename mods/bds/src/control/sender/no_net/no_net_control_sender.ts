import { system } from '@minecraft/server';
import type { Player } from '@minecraft/server';
import type { ControlAction } from '../../action';
import { ControlCodec } from '../../codec';
import type { ControlSender, ControlSendResult } from '../control_sender';

// No-net mode: encode the action into a silent bvc:ctl: playSound targeted at the
// acting player. The desktop proxy decodes it and emits a ServerBound ClientAction
// (the actor is the proxy session's authenticated player). No secret is embedded —
// /playsound is command-gated, so a regular player cannot forge one.
export class NoNetControlSender implements ControlSender {
  async send(action: ControlAction, actor: Player): Promise<ControlSendResult> {
    await this.playCtl(ControlCodec.encodeCtl(action), actor);
    // Fire-and-forget bus: there is no reply channel, so a group-create's share
    // code cannot be surfaced on this path.
    return { ok: true };
  }

  // The panel's scoped snapshot request: the proxy answers with !bvcs: rides.
  async requestSync(targets: string[], actor: Player): Promise<void> {
    await this.playCtl(ControlCodec.encodeSync(targets), actor);
  }

  private async playCtl(name: string, actor: Player): Promise<void> {
    // Yield to a fresh tick so playSound runs outside any read-only command
    // callback context (mirrors NoNetAudioSender).
    await this.nextTick();
    const c = actor.location;
    actor.playSound(name, {
      location: { x: c.x, y: c.y, z: c.z },
      volume: 0,
    });
  }

  private nextTick(): Promise<void> {
    return new Promise((resolve) => system.run(resolve));
  }
}
