import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { GroupName } from "../core/naming/GroupName";

/**
 * A stub draw. `GroupName` calls `rand` twice per attempt — once for the adjective, once for
 * the noun — so a sequence of two values per attempt pins exactly which pairs are drawn and
 * in what order, which is the only way to test the collision paths without waiting on chance.
 */
function pinned(...values: readonly number[]): (bound: number) => number {
  let at = 0;
  return () => values[at++ % values.length];
}

const FIRST = `${GroupName.ADJECTIVES[0]} ${GroupName.NOUNS[0]}`;
const SECOND = `${GroupName.ADJECTIVES[1]} ${GroupName.NOUNS[1]}`;

describe("GroupName.next", () => {
  it("draws both words from the lists", () => {
    const words = GroupName.next().split(" ");
    assert.equal(words.length, 2);
    assert.ok(GroupName.ADJECTIVES.includes(words[0]), `adjective: ${words[0]}`);
    assert.ok(GroupName.NOUNS.includes(words[1]), `noun: ${words[1]}`);
  });

  it("gives a free name straight back", () => {
    assert.equal(GroupName.next([], pinned(0, 0)), FIRST);
  });

  it("redraws when the first draw is already a group on this server", () => {
    assert.equal(GroupName.next([FIRST], pinned(0, 0, 1, 1)), SECOND);
  });

  /**
   * The pool is finite, so a crowded server has to end somewhere other than a loop. With every
   * draw landing on the same taken pair, the name is taken as given and numbered.
   */
  it("numbers the name when every draw is taken", () => {
    assert.equal(GroupName.next([FIRST], pinned(0, 0)), `${FIRST} 2`);
  });

  it("keeps counting when the numbered form is taken too", () => {
    assert.equal(GroupName.next([FIRST, `${FIRST} 2`], pinned(0, 0)), `${FIRST} 3`);
  });

  /**
   * Names arrive from the server as the user typed them. "obsidian ocelots" and " Obsidian
   * Ocelots " are the same row to a reader, so they have to be the same name to the draw.
   */
  it("matches a taken name regardless of case or surrounding space", () => {
    assert.equal(GroupName.next([`  ${FIRST.toUpperCase()}  `], pinned(0, 0)), `${FIRST} 2`);
  });
});

describe("GroupName.randomInt", () => {
  it("stays below the bound", () => {
    for (let i = 0; i < 2000; i++) {
      const value = GroupName.randomInt(64);
      assert.ok(Number.isInteger(value), `not an integer: ${value}`);
      assert.ok(value >= 0 && value < 64, `out of range: ${value}`);
    }
  });

  it("reaches both ends of the range", () => {
    const seen = new Set<number>();
    for (let i = 0; i < 4000; i++) seen.add(GroupName.randomInt(3));
    assert.deepEqual([...seen].sort(), [0, 1, 2]);
  });

  /**
   * A bound of zero has no valid answer, and the rejection loop would spin forever looking for
   * one. Failing loudly is the difference between a bug report and a frozen window.
   */
  it("refuses a bound below one rather than looping", () => {
    assert.throws(() => GroupName.randomInt(0), RangeError);
    assert.throws(() => GroupName.randomInt(-4), RangeError);
  });
});

describe("GroupName word lists", () => {
  /**
   * A repeated word is invisible on inspection and halves nothing loudly: it just makes one
   * name twice as likely as its neighbours and quietly shrinks the pool below 4,096.
   */
  it("holds no duplicates", () => {
    for (const list of [GroupName.ADJECTIVES, GroupName.NOUNS]) {
      assert.equal(new Set(list).size, list.length, `duplicate in ${list.slice(0, 3)}...`);
    }
  });

  it("never composes a name long enough to wrap a group row", () => {
    for (const adjective of GroupName.ADJECTIVES) {
      for (const noun of GroupName.NOUNS) {
        const name = `${adjective} ${noun}`;
        assert.ok(name.length <= 26, `${name} is ${name.length} characters`);
      }
    }
  });
});
