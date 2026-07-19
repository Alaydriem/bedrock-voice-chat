// The ergonomic, BDS-internal shape a `/bvc` command produces. This is NOT the wire
// type — `ControlCodec` (codec.ts) maps it to the `bvc:ctl:` string and the
// `/api/control` JSON, whose shapes are owned + contract-tested in the Rust `common`
// crate. This mod is standalone (no shared types with common) by design.
export type ControlAction =
  | { kind: 'mute'; on: boolean }
  | { kind: 'deafen'; on: boolean }
  | { kind: 'record'; on: boolean }
  | { kind: 'volume'; target: string; value: number }
  | { kind: 'hear'; target: string; on: boolean }
  | { kind: 'group-create' }
  | { kind: 'group-join'; channel: string }
  | { kind: 'group-leave' };
