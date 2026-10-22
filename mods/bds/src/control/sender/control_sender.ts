import type { Player } from '@minecraft/server';
import type { ControlAction } from '../action';

// Outcome of submitting a ControlAction. `groupCode` is only present on a
// successful net-mode group-create — the server's reply carries the new group's
// share code (the nanoid); the no-net path has no reply channel.
export interface ControlSendResult {
  ok: boolean;
  groupCode?: string;
  // A protected join: handed to the player's BVC app, or no app session to hand it to.
  // Net mode only; no-net mode has no reply channel.
  outcome?: 'forwarded' | 'no_session';
}

export interface ControlSender {
  send(action: ControlAction, actor: Player): Promise<ControlSendResult>;
}
