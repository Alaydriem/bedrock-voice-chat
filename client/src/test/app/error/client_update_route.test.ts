import { describe, expect, it, vi } from "vitest";
import { ClientUpdateRoute } from "../../../js/app/error/ClientUpdateRoute";

describe("ClientUpdateRoute", () => {
    /**
     * A device with one saved server never sees the server list, so its preflight never offers
     * the update. VER01 is where that device lands, and the only screen it can install from.
     */
    it("sends a client too old for its server to the install screen when an update exists", async () => {
        const route = new ClientUpdateRoute(async () => "1.0.0-beta.24");

        expect(await route.replacementFor("VER01")).toBe("/error?code=UPD01&version=1.0.0-beta.24");
    });

    it("keeps VER01 when no newer build has been published", async () => {
        const route = new ClientUpdateRoute(async () => null);

        expect(await route.replacementFor("VER01")).toBeNull();
    });

    // Mobile has no check_for_updates command, so the invoke rejects. Its copy already says
    // to fetch the new client from the store, which is the right answer there.
    it("keeps VER01 when this build cannot update itself", async () => {
        const route = new ClientUpdateRoute(async () => {
            throw new Error("command check_for_updates not found");
        });

        expect(await route.replacementFor("VER01")).toBeNull();
    });

    it("never checks for an update behind any other fault", async () => {
        const check = vi.fn(async () => "1.0.0-beta.24");
        const route = new ClientUpdateRoute(check);

        expect(await route.replacementFor("VER02")).toBeNull();
        expect(await route.replacementFor("UPD01")).toBeNull();
        expect(await route.replacementFor(null)).toBeNull();
        expect(check).not.toHaveBeenCalled();
    });
});
