import { world, system } from '@minecraft/server';
import type { Player } from '@minecraft/server';
import type { AudioPlayerState } from '../player_state';
import type { AudioSender } from './audio_sender';
import { Dimension } from '../../dto/dimension';
import { JukeboxBusProtocol } from './protocol';

export class NoNetAudioSender implements AudioSender {
  async start(state: AudioPlayerState, actor: Player | null): Promise<void> {
    await this.nextTick();

    const fulfiller = this.resolveFulfiller(state, actor);
    if (!fulfiller) {
      console.warn('[BVC] No fulfiller for jukebox play; audio not started');
      return;
    }

    const c = state.coordinates;
    fulfiller.playSound(
      JukeboxBusProtocol.playName(state.audioId, Dimension.of(state.dimensionId)),
      {
        location: { x: c.x, y: c.y, z: c.z },
        volume: 0.1,
      },
    );
  }

  async stop(state: AudioPlayerState, actor: Player | null): Promise<void> {
    await this.nextTick();

    const fulfiller = this.resolveFulfiller(state, actor);
    if (!fulfiller) {
      return;
    }

    const c = state.coordinates;
    fulfiller.playSound(JukeboxBusProtocol.ejectName(Dimension.of(state.dimensionId)), {
      location: { x: c.x, y: c.y, z: c.z },
      volume: 0.1,
    });
  }

  // Yields to a fresh tick so the chat send runs outside the block-component
  // callback context, where mutating/side-effecting APIs are not permitted.
  private nextTick(): Promise<void> {
    return new Promise((resolve) => system.run(resolve));
  }

  private resolveFulfiller(
    state: AudioPlayerState,
    actor: Player | null,
  ): Player | null {
    if (actor) {
      return actor;
    }

    return this.closestPlayer(state);
  }

  private closestPlayer(state: AudioPlayerState): Player | null {
    const c = state.coordinates;

    let best: Player | null = null;
    let bestDistSq = Number.POSITIVE_INFINITY;
    let bestId = '';

    for (const p of world.getAllPlayers()) {
      if (!Dimension.of(p.dimension.id).equals(Dimension.of(state.dimensionId))) {
        continue;
      }

      const dx = p.location.x - (c.x + 0.5);
      const dy = p.location.y - (c.y + 0.5);
      const dz = p.location.z - (c.z + 0.5);
      const distSq = dx * dx + dy * dy + dz * dz;

      if (distSq < bestDistSq || (distSq === bestDistSq && p.id < bestId)) {
        best = p;
        bestDistSq = distSq;
        bestId = p.id;
      }
    }

    return best;
  }
}
