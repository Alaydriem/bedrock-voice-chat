import type { Player } from '@minecraft/server';
import type { ControlAction } from '../action';

export interface ControlSender {
  send(action: ControlAction, actor: Player): Promise<void>;
}
