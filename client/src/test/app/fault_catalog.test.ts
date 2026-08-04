import { describe, expect, it } from "vitest";
import FaultCatalog from "../../js/app/error/FaultCatalog";

/**
 * The error route is the last screen someone sees before they give up, so its two failure
 * modes are both terminal: a code nobody can look up, and a screen with no way off it.
 *
 * Every definition is checked, not a sample. A catalog is exactly the kind of data where a
 * fifteenth entry gets added by hand months later and only that entry is wrong.
 */

const DEFINITIONS = Object.values(FaultCatalog.DEFINITIONS);

describe("resolving a code", () => {
  it("keeps an unrecognised code so it can still be looked up", () => {
    const resolved = FaultCatalog.resolve("KABOOM9");
    expect(resolved.code).toBe("KABOOM9");
    expect(resolved.title).toBe(FaultCatalog.DEFINITIONS.DEFAULT.title);
  });

  it("falls back when no code was given at all", () => {
    expect(FaultCatalog.resolve(null).code).toBe("ERROR");
    expect(FaultCatalog.resolve("").code).toBe("ERROR");
  });
});

describe("dropping the server switch", () => {
  /**
   * With one server configured there is nowhere to switch to. Offering it anyway sends
   * someone who is already stuck to a list of one, which is where the report that the app
   * "does nothing" comes from.
   */
  it("leaves no action pointing at the server list", () => {
    for (const definition of DEFINITIONS) {
      const adjusted = FaultCatalog.withoutServerSwitch(definition);
      expect(adjusted.primaryAction.url, definition.code).not.toBe("/server");
      expect(adjusted.secondaryAction?.url, definition.code).not.toBe("/server");
    }
  });

  /**
   * A definition whose primary action was the switch — VER02, AGE01 — must not be left with
   * a button that goes nowhere. The update is the exception by design: its primary action
   * installs rather than navigates, which is why it carries no URL.
   */
  it("still leaves every screen a way off it", () => {
    for (const definition of DEFINITIONS) {
      const adjusted = FaultCatalog.withoutServerSwitch(definition);
      expect(adjusted.primaryAction.label, definition.code).toBeTruthy();
      if (definition.code !== FaultCatalog.UPDATE) {
        expect(adjusted.primaryAction.url, definition.code).toBeTruthy();
      }
    }
  });

  it("does not disturb a definition that never offered it", () => {
    const perm1 = FaultCatalog.DEFINITIONS.PERM1;
    expect(FaultCatalog.withoutServerSwitch(perm1)).toEqual(perm1);
  });
});

describe("naming the version", () => {
  it("puts it in the update's copy and caption", () => {
    const named = FaultCatalog.withVersion(FaultCatalog.DEFINITIONS.UPD01, "1.0.0-beta.9");
    expect(named.message).toContain("v1.0.0-beta.9");
    expect(named.caption).toBe("v1.0.0-beta.9");
  });

  /**
   * `version` arrives on the query string, so any code can carry one. Rewriting a
   * connection failure's copy to announce a new version would be nonsense on the screen
   * that matters most.
   */
  it("ignores it on every other code", () => {
    const conn = FaultCatalog.DEFINITIONS.CONN01;
    expect(FaultCatalog.withVersion(conn, "1.0.0-beta.9")).toEqual(conn);
  });

  it("leaves the update alone when no version came through", () => {
    const update = FaultCatalog.DEFINITIONS.UPD01;
    expect(FaultCatalog.withVersion(update, null)).toEqual(update);
  });
});

/**
 * The chip replaces the eyebrow to say the app is not reporting a fault. Putting one on a
 * break tells someone nothing is wrong on the screen explaining what is, and omitting one
 * from `warn` or `ok` states the opposite of what those screens mean.
 */
describe("the severity chip", () => {
  it("appears on exactly the screens that are not reporting a break", () => {
    for (const definition of DEFINITIONS) {
      const shouldHaveChip = definition.severity !== "bad";
      expect(definition.chip !== undefined, `${definition.code} (${definition.severity})`).toBe(
        shouldHaveChip,
      );
    }
  });

  it("marks the update as the only thing here that is good news", () => {
    const good = DEFINITIONS.filter((definition) => definition.severity === "ok");
    expect(good.map((definition) => definition.code)).toEqual([FaultCatalog.UPDATE]);
  });
});
