import { describe, expect, it } from "vitest";
import FaultCatalog from "../../../js/app/error/FaultCatalog";
import { Icons } from "../../../radial/core/icons/Icons";

/**
 * The two codes a failed credential write lands on. Asserted against DEFINITIONS rather than
 * through resolve(): resolve keeps an unrecognised code and pairs it with the DEFAULT copy, so a
 * missing definition would still answer with the code that was asked for.
 */
describe("keyring fault definitions", () => {
  it("defines both keyring fault codes", () => {
    expect(FaultCatalog.DEFINITIONS.AUTH03).toBeDefined();
    expect(FaultCatalog.DEFINITIONS.AUTH04).toBeDefined();
  });

  it("uses icons that exist", () => {
    expect(Icons.has(FaultCatalog.DEFINITIONS.AUTH03.icon)).toBe(true);
    expect(Icons.has(FaultCatalog.DEFINITIONS.AUTH04.icon)).toBe(true);
  });

  /**
   * AUTH04 is the only one of the two with a remedy, and the remedy is the whole reason it is a
   * separate code. Copy that stops naming the keyring makes it indistinguishable from AUTH03.
   */
  it("AUTH04 names the keyring in its message", () => {
    expect(FaultCatalog.DEFINITIONS.AUTH04.message.toLowerCase()).toContain("keyring");
  });

  /**
   * A storage failure that cannot be retried away is a break; one waiting on a person to create a
   * keyring is not. The catalogue-wide chip invariant keys off exactly this distinction.
   */
  it("separates the break from the one waiting on a person", () => {
    expect(FaultCatalog.DEFINITIONS.AUTH03.severity).toBe("bad");
    expect(FaultCatalog.DEFINITIONS.AUTH04.severity).toBe("warn");
  });
});
