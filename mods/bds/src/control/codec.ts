import type { ControlAction } from './action';

// Maps a BDS-internal ControlAction to the two wire shapes the Rust server accepts:
//   - encodeCtl:          the `bvc:ctl:` playSound string (no-net path, decoded by
//                         common's CtlCodec in the desktop proxy)
//   - toClientActionJson: the JSON body for POST /api/control (net path), matching
//                         common's ClientAction/ClientActionType serde
// Both shapes are owned + contract-tested in the Rust `common` crate; a drift here
// silently desyncs this standalone mod from the server.
export class ControlCodec {
  private static readonly PREFIX = 'bvc:ctl:';

  static encodeCtl(a: ControlAction): string {
    const p = ControlCodec.PREFIX;
    switch (a.kind) {
      case 'mute':
        return `${p}mute:${a.on ? 1 : 0}`;
      case 'deafen':
        return `${p}deafen:${a.on ? 1 : 0}`;
      case 'record':
        return `${p}record:${a.on ? 1 : 0}`;
      case 'volume':
        return `${p}vol:${a.target}:${Math.round(a.value)}`;
      case 'hear':
        return `${p}hear:${a.target}:${a.on ? 1 : 0}`;
      case 'group-create':
        return `${p}group:create`;
      case 'group-join':
        return `${p}group:join:${a.channel}`;
      case 'group-leave':
        return `${p}group:leave`;
    }
  }

  static toClientActionJson(a: ControlAction, actorId: string): unknown {
    return { id: actorId, action: ControlCodec.actionJson(a) };
  }

  // Unit variants serialize as bare strings; struct/tuple variants as { Variant: ... }.
  private static actionJson(a: ControlAction): unknown {
    switch (a.kind) {
      case 'mute':
        return { SetMuted: a.on };
      case 'deafen':
        return { SetDeafened: a.on };
      case 'record':
        return { SetRecording: a.on };
      case 'volume':
        return { SetVolume: { target: a.target, volume: a.value / 100 } };
      case 'hear':
        return { SetHeard: { target: a.target, muted: !a.on } };
      case 'group-create':
        return 'CreateGroup';
      case 'group-join':
        return { JoinGroup: { channel: a.channel } };
      case 'group-leave':
        return 'LeaveGroup';
    }
  }
}
