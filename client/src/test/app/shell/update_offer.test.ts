import { describe, expect, it, vi } from "vitest";
import { get } from "svelte/store";
import { UpdateOffer, type SessionFlags } from "../../../js/app/shell/UpdateOffer";

function session(): SessionFlags {
    const values = new Map<string, string>();
    return {
        getItem: (key) => values.get(key) ?? null,
        setItem: (key, value) => void values.set(key, value),
    };
}

describe("UpdateOffer", () => {
    it("offers a published update as soon as the check returns it", async () => {
        const offer = new UpdateOffer(async () => "1.0.524", session());
        await offer.check();
        expect(get(offer.offer)).toBe("1.0.524");
    });

    it("offers nothing when this build is current", async () => {
        const offer = new UpdateOffer(async () => null, session());
        await offer.check();
        expect(get(offer.offer)).toBeNull();
    });

    // Mobile has no check_for_updates command, and a manifest host can be down. Neither is
    // something to interrupt a launch about.
    it("offers nothing when the check fails", async () => {
        const offer = new UpdateOffer(async () => {
            throw new Error("command check_for_updates not found");
        }, session());
        await offer.check();
        expect(get(offer.offer)).toBeNull();
    });

    it("leads to the install screen for the offered version", async () => {
        const offer = new UpdateOffer(async () => "1.0.524", session());
        await offer.check();
        expect(offer.accept()).toBe("/error?code=UPD01&version=1.0.524");
        expect(get(offer.offer)).toBeNull();
    });

    it("has nowhere to lead before an update is offered", () => {
        const offer = new UpdateOffer(async () => "1.0.524", session());
        expect(offer.accept()).toBeNull();
    });

    /**
     * "Not now" holds until the app is next launched. Several screens replace the document
     * rather than navigating within it, and each replacement builds a fresh offer — the
     * dismissal has to outlive that, or the dialog returns on the next screen.
     */
    it("stays dismissed for the rest of the session, across reloads", async () => {
        const flags = session();
        const check = vi.fn(async () => "1.0.524");

        const first = new UpdateOffer(check, flags);
        await first.check();
        first.dismiss();
        expect(get(first.offer)).toBeNull();

        const afterReload = new UpdateOffer(check, flags);
        await afterReload.check();
        expect(get(afterReload.offer)).toBeNull();
        expect(check).toHaveBeenCalledTimes(1);
    });

    it("offers again in a new session", async () => {
        const dismissed = new UpdateOffer(async () => "1.0.524", session());
        await dismissed.check();
        dismissed.dismiss();

        const nextLaunch = new UpdateOffer(async () => "1.0.524", session());
        await nextLaunch.check();
        expect(get(nextLaunch.offer)).toBe("1.0.524");
    });

    // A webview with storage blocked still gets the offer; it only loses the memory of a
    // dismissal across reloads.
    it("still offers when session storage is unavailable", async () => {
        const offer = new UpdateOffer(async () => "1.0.524", null);
        await offer.check();
        expect(get(offer.offer)).toBe("1.0.524");
        offer.dismiss();
        expect(get(offer.offer)).toBeNull();
    });
});
