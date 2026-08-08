import { system, world } from '@minecraft/server';
import type { Block } from '@minecraft/server';
import { Coordinates } from '../dto';
import type { AudioPlayerManager } from '../audio/player_manager';

interface JukeboxTarget {
  readonly block: Block;
  readonly coordinates: Coordinates;
  readonly locationKey: string;
}

// Drives a jukebox from the BDS console, with no player, no item, and no hopper.
//
// The physical route needs a held disc and a hand to hold it, so it cannot be reached from a
// console. This calls the same manager methods the block component calls, which is what makes an
// automated run exercise the code the game exercises.
//
//   scriptevent bvc:insert 019fd3a8-74db-7a60-a8f7-d774d6af8e04
//   scriptevent bvc:insert 019fd3a8-74db-7a60-a8f7-d774d6af8e04 12 70 -30
//   scriptevent bvc:eject
export class JukeboxTestCommands {
  private static readonly DEFAULT_LOCATION = new Coordinates(0, 64, 0);
  private static readonly DIMENSION = 'minecraft:overworld';
  private static readonly BLOCK_ID = 'bvc:audio_player';

  private readonly audioManager: AudioPlayerManager;
  private readonly getWorldUuid: () => string;

  constructor(audioManager: AudioPlayerManager, getWorldUuid: () => string) {
    this.audioManager = audioManager;
    this.getWorldUuid = getWorldUuid;
  }

  register(): void {
    system.afterEvents.scriptEventReceive.subscribe(
      (event) => {
        if (event.id === 'bvc:insert') {
          this.onInsert(event.message.trim());
        } else if (event.id === 'bvc:eject') {
          this.onEject(event.message.trim());
        }
      },
      { namespaces: ['bvc'] },
    );
    console.info(
      '[BVC jukebox-test] ready — scriptevent bvc:insert <audio_id> [x y z] | scriptevent bvc:eject [x y z]',
    );
  }

  private onInsert(message: string): void {
    const parts = JukeboxTestCommands.words(message);
    const audioId = parts[0];
    if (!audioId) {
      console.error(
        '[BVC jukebox-test] usage: scriptevent bvc:insert <audio_id> [x y z]',
      );
      return;
    }

    const target = this.resolveTarget(parts.slice(1));
    if (!target) {
      return;
    }

    if (this.audioManager.hasDisc(target.locationKey)) {
      console.warn(
        `[BVC jukebox-test] a disc is already in the jukebox at ${target.locationKey}; eject first`,
      );
      return;
    }

    this.audioManager.insertDisc(
      target.locationKey,
      audioId,
      JukeboxTestCommands.DIMENSION,
      target.coordinates,
      this.getWorldUuid(),
      null,
    );
    this.setPlaying(target.block, true);
    console.info(
      `[BVC jukebox-test] inserted ${audioId} at ${target.locationKey}`,
    );
  }

  private onEject(message: string): void {
    const target = this.resolveTarget(JukeboxTestCommands.words(message));
    if (!target) {
      return;
    }

    if (!this.audioManager.hasDisc(target.locationKey)) {
      console.warn(`[BVC jukebox-test] no disc at ${target.locationKey}`);
      return;
    }

    const audioId = this.audioManager.ejectDisc(target.locationKey, null);
    this.setPlaying(target.block, false);
    console.info(
      `[BVC jukebox-test] ejected ${audioId ?? 'unknown'} at ${target.locationKey}`,
    );
  }

  // The block has to be loaded and has to be ours: inserting against an arbitrary block would
  // leave manager state pointing at a jukebox that does not exist, and nothing would ever eject
  // it.
  private resolveTarget(coords: string[]): JukeboxTarget | null {
    const coordinates = JukeboxTestCommands.parseCoordinates(coords);
    if (!coordinates) {
      console.error('[BVC jukebox-test] coordinates must be three integers');
      return null;
    }

    const dimension = world.getDimension(JukeboxTestCommands.DIMENSION);
    let block: Block | undefined;
    try {
      block = dimension.getBlock({
        x: coordinates.x,
        y: coordinates.y,
        z: coordinates.z,
      });
    } catch (e) {
      console.error(
        `[BVC jukebox-test] cannot read the block, chunk unloaded? ${e}`,
      );
      return null;
    }

    if (!block) {
      console.error(
        `[BVC jukebox-test] no block at (${coordinates.x},${coordinates.y},${coordinates.z}); is the chunk loaded?`,
      );
      return null;
    }

    if (block.typeId !== JukeboxTestCommands.BLOCK_ID) {
      console.error(
        `[BVC jukebox-test] block at (${coordinates.x},${coordinates.y},${coordinates.z}) is ${block.typeId}, not ${JukeboxTestCommands.BLOCK_ID}`,
      );
      return null;
    }

    return {
      block,
      coordinates,
      locationKey: this.audioManager.locationKey(
        this.getWorldUuid(),
        coordinates,
      ),
    };
  }

  private static words(message: string): string[] {
    return message.split(/\s+/).filter((part) => part.length > 0);
  }

  private static parseCoordinates(parts: string[]): Coordinates | null {
    if (parts.length === 0) {
      return JukeboxTestCommands.DEFAULT_LOCATION;
    }
    if (parts.length !== 3) {
      return null;
    }

    const parsed = parts.map((part) => Number.parseInt(part, 10));
    if (parsed.some((value) => Number.isNaN(value))) {
      return null;
    }

    return new Coordinates(parsed[0], parsed[1], parsed[2]);
  }

  private setPlaying(block: Block, playing: boolean): void {
    try {
      block.setPermutation(
        block.permutation.withState('bvc:playing' as never, playing),
      );
    } catch (e) {
      console.error('[BVC jukebox-test] failed to set bvc:playing:', e);
    }
  }
}
