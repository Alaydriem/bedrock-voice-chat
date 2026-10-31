import type { Dimension } from '../../dto/dimension';

// Wire contract with the client proxy's PlaySoundHandler
// (client/src-tauri/src/bedrock/proxy/session/handlers/play_sound.rs).
// Keep these prefixes in sync with the proxy's PLAY / EJECT constants.
export class JukeboxBusProtocol {
  static readonly PLAY = 'bvc:play:';
  static readonly EJECT = 'bvc:eject:';

  // The proxy splits on the first ':' after the prefix, so the dimension id may carry
  // colons of its own.
  static playName(audioId: string, dimension: Dimension): string {
    return `${JukeboxBusProtocol.PLAY}${audioId}:${dimension.id}`;
  }

  static ejectName(dimension: Dimension): string {
    return `${JukeboxBusProtocol.EJECT}${dimension.id}`;
  }
}
