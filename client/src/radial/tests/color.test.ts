import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { Color } from "../core/color/Color";

describe("Color", () => {
  it("parses rgb() as well as hex", () => {
    // The regression this exists for: the prototype's mix() returned rgb() while its
    // rgba() parsed only hex, so every blended ring bar came out black. Any parser that
    // reads rgb() as zero reintroduces it.
    assert.deepEqual([...Color.channels("rgb(130, 57, 216)")], [130, 57, 216]);
    assert.deepEqual([...Color.channels("rgba(130,57,216,0.5)")], [130, 57, 216]);
    assert.deepEqual([...Color.channels("#8239d8")], [130, 57, 216]);
    assert.deepEqual([...Color.channels("#83d")], [136, 51, 221]);
  });

  it("survives a mix fed straight back into rgba", () => {
    const blended = Color.mix("#6a4f96", "#21d8d8", 0.5);
    const applied = Color.rgba(blended, 0.62);
    assert.notEqual(applied, "rgba(0,0,0,0.62)");
    assert.deepEqual([...Color.channels(applied)], [...Color.channels(blended)]);
  });

  it("returns each endpoint at the ends of a mix", () => {
    assert.equal(Color.mix("#000000", "#ffffff", 0), "#000000");
    assert.equal(Color.mix("#000000", "#ffffff", 1), "#ffffff");
  });

  it("falls back to white rather than black on unparseable input", () => {
    // Black would silently disappear against the violet ground; white is visibly wrong,
    // which is what an unparseable colour should be.
    assert.deepEqual([...Color.channels("not a colour")], [255, 255, 255]);
  });

  it("measures the contrast ratios the palette is built on", () => {
    const ground = "#1c1132";
    // Body text and the mono labels are the two pairs the whole dark palette rests on.
    assert.ok(Color.contrast("#d6cbea", ground) >= 4.5, "body text below AA");
    assert.ok(Color.contrast("#b3a4d0", ground) >= 4.5, "mono labels below AA");
    assert.ok(Color.contrast("#fbf8ff", ground) >= 7, "display text below AAA");
    for (const semantic of ["#5ce383", "#ffcf4d", "#ff8266"]) {
      assert.ok(Color.contrast(semantic, ground) >= 4.5, `${semantic} below AA`);
    }
  });

  it("is symmetric in contrast", () => {
    assert.equal(
      Color.contrast("#d6cbea", "#1c1132").toFixed(6),
      Color.contrast("#1c1132", "#d6cbea").toFixed(6),
    );
  });
});
