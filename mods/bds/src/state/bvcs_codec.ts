// Decoder for the no-net reverse-ride grammar the desktop proxy injects as
// serverbound chat. Mirrors common/src/structs/control/bvcs_codec.rs — the
// common-side goldens pin the strings; keep both ends in sync:
//
//   !bvcs:<seq>:q:m=<0|1>;d=<0|1>;r=<0|1>;g=<group|->   self-state snapshot
//   !bvcs:<seq>:p:t=<target>;v=<percent>;h=<0|1>        one player preference

export interface BvcsQueryState {
  kind: 'q';
  muted: boolean;
  deafened: boolean;
  recording: boolean;
  group: string | null;
}

export interface BvcsPreference {
  kind: 'p';
  target: string;
  volume: number;
  heard: boolean;
}

export type BvcsMessage = BvcsQueryState | BvcsPreference;

const BVCS_PREFIX = '!bvcs:';

export class BvcsCodec {
  static isBvcs(message: string): boolean {
    return message.startsWith(BVCS_PREFIX);
  }

  static decode(message: string): BvcsMessage | null {
    if (!message.startsWith(BVCS_PREFIX)) {
      return null;
    }
    const rest = message.slice(BVCS_PREFIX.length);
    const first = rest.indexOf(':');
    if (first < 0) {
      return null;
    }
    const second = rest.indexOf(':', first + 1);
    if (second < 0) {
      return null;
    }
    if (!/^\d+$/.test(rest.slice(0, first))) {
      return null;
    }

    const kind = rest.slice(first + 1, second);
    const fields = new Map<string, string>();
    for (const kv of rest.slice(second + 1).split(';')) {
      const eq = kv.indexOf('=');
      if (eq > 0) {
        fields.set(kv.slice(0, eq), kv.slice(eq + 1));
      }
    }

    if (kind === 'q') {
      const m = fields.get('m');
      const d = fields.get('d');
      const r = fields.get('r');
      const g = fields.get('g');
      if (m === undefined || d === undefined || r === undefined) {
        return null;
      }
      return {
        kind: 'q',
        muted: m === '1',
        deafened: d === '1',
        recording: r === '1',
        group: g !== undefined && g !== '-' ? g : null,
      };
    }

    if (kind === 'p') {
      const t = fields.get('t');
      const v = fields.get('v');
      const h = fields.get('h');
      if (!t || v === undefined || h === undefined) {
        return null;
      }
      const percent = Number(v);
      if (!Number.isFinite(percent)) {
        return null;
      }
      return {
        kind: 'p',
        target: t,
        volume: percent / 100,
        heard: h === '1',
      };
    }

    return null;
  }
}
