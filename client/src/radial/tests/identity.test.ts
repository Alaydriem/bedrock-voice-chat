import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { ServerGlyph } from "../core/glyph/ServerGlyph";
import { MarkData } from "../core/mark/MarkData";
import { PlayerHue } from "../core/sources/PlayerHue";
import { PositionalSource } from "../core/sources/PositionalSource";

describe("ServerGlyph", () => {
  it("is the same glyph on every client that knows the name", () => {
    // Determinism is the whole feature: two people looking at the same server must see
    // the same tile without anything being transmitted.
    const a = ServerGlyph.of("bvc.alaydriem.com");
    const b = ServerGlyph.of("bvc.alaydriem.com");
    assert.equal(a.hue, b.hue);
    assert.deepEqual(a.bits, b.bits);
  });

  it("gives different names different glyphs", () => {
    const names = ["bvc.alaydriem.com", "voice.hearthhold.net", "bvc.tinyaxolotl.gg", "a", "b"];
    const seen = new Set(names.map((n) => JSON.stringify(ServerGlyph.of(n).bits)));
    assert.equal(seen.size, names.length);
  });

  it("takes its hue from a column of the mark", () => {
    for (const name of ["a", "bvc.example.com", "voice.hearthhold.net"]) {
      const glyph = ServerGlyph.of(name);
      assert.ok(glyph.hueIndex >= 0 && glyph.hueIndex < MarkData.COLS);
      assert.equal(glyph.hue, MarkData.hueAt(glyph.hueIndex));
    }
  });

  it("is mirrored about the centre column", () => {
    // Symmetry is what makes an arbitrary hash read as a mark rather than as noise.
    const { bits } = ServerGlyph.of("bvc.alaydriem.com");
    for (let row = 0; row < ServerGlyph.GRID; row++) {
      for (let col = 0; col < 2; col++) {
        assert.equal(
          bits[row * ServerGlyph.GRID + col],
          bits[row * ServerGlyph.GRID + (4 - col)],
          `row ${row} column ${col}`,
        );
      }
    }
  });

  // The other half of the drift guard. `common/websocket-types/tests/glyph/server.rs` asserts
  // these same values, so a controller draws the tile the desktop draws. Nothing at runtime can
  // detect the two derivations diverging — the glyphs would simply stop matching.
  //
  // The non-ASCII rows are the ones that matter most: an implementation hashing bytes rather
  // than UTF-16 code units passes every ASCII case and fails only these.
  it("agrees with the Rust derivation on fixed vectors", () => {
    const cases: [string, string, number, string][] = [
      ["bvc.alaydriem.com", "#f9871d", 17, "1b57eb5"],
      ["voice.hearthhold.net", "#f67414", 18, "15faa31"],
      ["a", "#f8e434", 14, "0a212a4"],
      ["Ops", "#466cf3", 4, "1b013fb"],
      ["café", "#466cf3", 4, "1bfc7e4"],
      ["日本", "#8238d8", 1, "1bfa884"],
    ];

    for (const [name, hue, hueIndex, pattern] of cases) {
      const glyph = ServerGlyph.of(name);
      let value = 0n;
      for (let i = 0; i < 25; i++) if (glyph.bits[i]) value |= 1n << BigInt(i);

      assert.equal(glyph.hue, hue, `hue for ${name}`);
      assert.equal(glyph.hueIndex, hueIndex, `hue index for ${name}`);
      assert.equal(value.toString(16).padStart(7, "0"), pattern, `pattern for ${name}`);
    }
  });
});

describe("PlayerHue", () => {
  it("keys on the certificate CN form, so one player is one colour everywhere", () => {
    assert.equal(PlayerHue.forPlayer("minecraft", "Alaydriem"), PlayerHue.of("minecraft:Alaydriem"));
  });

  it("ignores case, because a gamertag's casing is not its identity", () => {
    assert.equal(PlayerHue.of("minecraft:Alaydriem"), PlayerHue.of("MINECRAFT:alaydriem"));
  });

  it("separates the same name under different game prefixes", () => {
    // The hue is keyed on the whole canonical identity, so the same gamertag under two
    // prefixes is two identities as far as certificates go.
    const a = PlayerHue.columnOf("minecraft:Alaydriem");
    const b = PlayerHue.columnOf("othergame:Alaydriem");
    assert.notEqual(a, b);
  });

  it("only ever returns a colour from the mark", () => {
    const palette = new Set(MarkData.COLUMNS.map((c) => c[2]));
    for (const name of ["a", "b", "Petra", "Juno", "Kestrel", "Moth", "Wren"]) {
      assert.ok(palette.has(PlayerHue.forPlayer("minecraft", name)));
    }
  });
});

describe("PositionalSource", () => {
  it("is silent at and beyond the range", () => {
    assert.equal(PositionalSource.falloff(PositionalSource.RANGE), 0);
    assert.equal(PositionalSource.falloff(PositionalSource.RANGE + 10), 0);
  });

  it("is full volume on top of you", () => {
    assert.equal(PositionalSource.falloff(0), 1);
  });

  it("falls off faster than linearly", () => {
    // Someone twenty metres away should be much quieter, not slightly quieter.
    const half = PositionalSource.falloff(PositionalSource.RANGE / 2);
    assert.ok(half < 0.5, `expected sub-linear falloff, got ${half}`);
    assert.ok(Math.abs(half - 0.25) < 1e-9);
  });

  it("drops a voice that is out of range rather than placing it silently", () => {
    const out = PositionalSource.toRingSource(
      { bearing: 0, distance: PositionalSource.RANGE + 1, hue: "#21d8d8" },
      1,
    );
    assert.equal(out, null);
  });

  it("exempts a channel member from distance entirely", () => {
    // Channels are full volume at any distance; that is what they are for.
    const source = PositionalSource.inChannel(1.2, "#aee236", 0.8);
    assert.ok(source);
    assert.equal(source.volume, 0.8);
  });

  it("never places a voice louder than full", () => {
    const source = PositionalSource.toRingSource({ bearing: 0, distance: 0, hue: "#fff", gain: 1.5 }, 1);
    assert.ok(source);
    assert.ok(source.volume <= 1);
  });
});
