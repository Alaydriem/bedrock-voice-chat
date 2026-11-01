// Wire contract with the client proxy's PlaySoundHandler
// (client/src-tauri/src/bedrock/proxy/session/handlers/play_sound.rs).
// Keep these prefixes in sync with the proxy's PLAY / EJECT constants.
export class JukeboxBusProtocol {
  static readonly PLAY = 'bvc:play:';
  static readonly EJECT = 'bvc:eject:';
}
