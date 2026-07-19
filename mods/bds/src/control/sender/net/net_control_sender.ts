import type { Player } from '@minecraft/server';
import { httpClient } from '../../../net';
import type { ServerAdminConfig } from '../../../config';
import type { ControlAction } from '../../action';
import { ControlCodec } from '../../codec';
import type { ControlSender } from '../control_sender';

// Net mode: POST the ClientAction to the BVC server. The actor is attributed from
// the in-game player who ran the command (the token gates the route so players
// cannot call it directly).
export class NetControlSender implements ControlSender {
  // Warn only on reachability transitions, not on every failed request — otherwise a
  // brief server outage plus a few commands floods the console. `null` = unknown yet.
  private reachable: boolean | null = null;

  constructor(private readonly config: ServerAdminConfig) {}

  async send(action: ControlAction, actor: Player): Promise<void> {
    const body = JSON.stringify(
      ControlCodec.toClientActionJson(action, actor.name),
    );

    try {
      const response = await httpClient.request(
        `${this.config.bvcServer}/api/control`,
        'Post',
        body,
        [
          ['Content-Type', 'application/json'],
          ['X-MC-Access-Token', this.config.accessToken],
          ['Accept', 'application/json'],
        ],
        5,
      );

      const ok = !!response && response.status >= 200 && response.status < 300;
      if (ok) {
        this.markReachable();
      } else {
        const detail = response ? `status ${response.status}` : 'no response';
        this.markUnreachable(detail);
      }
    } catch (e) {
      this.markUnreachable(e instanceof Error ? e.message : String(e));
    }
  }

  private markUnreachable(detail: string): void {
    if (this.reachable !== false) {
      console.warn(
        `[BVC] Control server unreachable (${detail}); suppressing further warnings until it recovers`,
      );
    }
    this.reachable = false;
  }

  private markReachable(): void {
    if (this.reachable === false) {
      console.info('[BVC] Control server reachable again');
    }
    this.reachable = true;
  }
}
