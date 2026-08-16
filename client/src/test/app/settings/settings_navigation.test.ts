import { beforeEach, describe, expect, it, vi } from "vitest";

import { SettingsNavigation } from "../../../js/app/settings/SettingsNavigation";

// Typed with the parameters it stands in for, so `mock.calls` carries them: an
// argument-less stub records every call as an empty tuple and the assertions below
// cannot read the href out of it.
const navigate = vi.fn(async (_href: string, _opts?: { replaceState?: boolean }) => {});
const pop = vi.fn((_delta: number) => {});

/** A navigator for one platform. `mobile` is fixed per test, as it is per device. */
function nav(mobile: boolean): SettingsNavigation {
    return new SettingsNavigation(navigate, pop, () => mobile);
}

/** The hrefs handed to `navigate`, paired with whether each replaced or pushed. */
function moves(): Array<[string, boolean]> {
    return navigate.mock.calls.map(([href, opts]) => [href, Boolean(opts?.replaceState)]);
}

beforeEach(() => {
    navigate.mockClear();
    pop.mockClear();
});

describe("SettingsNavigation on desktop", () => {
    // Desktop has no list-only screen, so opening settings lands on a pane — the one the
    // gear has always opened, not the catalogue's unknown-pane fallback.
    it("opens on a pane with a single push", async () => {
        await nav(false).open();
        expect(moves()).toEqual([["/dashboard/settings/audio", false]]);
    });

    /**
     * The bug this exists to fix. Audio to Connect to About is three sibling moves, not
     * a descent, so it must not bury the way out three entries deep.
     */
    it("replaces on a sideways pane change", async () => {
        const settings = nav(false);
        await settings.open();
        navigate.mockClear();

        await settings.select("connect");
        await settings.select("about");

        expect(moves()).toEqual([
            ["/dashboard/settings/connect", true],
            ["/dashboard/settings/about", true],
        ]);
        expect(pop).not.toHaveBeenCalled();
    });

    it("pops the one entry it pushed when it leaves", async () => {
        const settings = nav(false);
        await settings.open();
        await settings.select("about");
        navigate.mockClear();

        await settings.exit("/dashboard/settings/about");

        expect(pop).toHaveBeenCalledWith(-1);
        expect(navigate).not.toHaveBeenCalled();
    });
});

describe("SettingsNavigation on mobile", () => {
    it("opens on the section list", async () => {
        await nav(true).open();
        expect(moves()).toEqual([["/dashboard/settings", false]]);
    });

    // Picking a pane on a phone is a descent to a second screen, so it pushes.
    it("pushes when a pane is picked", async () => {
        const settings = nav(true);
        await settings.open();
        navigate.mockClear();

        await settings.select("audio");

        expect(moves()).toEqual([["/dashboard/settings/audio", false]]);
    });

    it("climbs a pane to the section list", async () => {
        const settings = nav(true);
        await settings.open();
        await settings.select("audio");

        await settings.up("/dashboard/settings/audio");

        expect(pop).toHaveBeenCalledWith(-1);
    });

    it("climbs the section list to the dashboard", async () => {
        const settings = nav(true);
        await settings.open();

        await settings.up("/dashboard/settings");

        expect(pop).toHaveBeenCalledWith(-1);
    });

    it("pops both entries when it leaves from a pane", async () => {
        const settings = nav(true);
        await settings.open();
        await settings.select("audio");
        navigate.mockClear();

        await settings.exit("/dashboard/settings/audio");

        expect(pop).toHaveBeenCalledWith(-2);
        expect(navigate).not.toHaveBeenCalled();
    });

    /**
     * The dashboard's Connect shortcut removes a tap going in. It must not remove the
     * section list on the way out, so it pushes the whole chain rather than the
     * destination alone.
     */
    it("pushes the section list beneath a named pane", async () => {
        await nav(true).open("connect");
        expect(moves()).toEqual([
            ["/dashboard/settings", false],
            ["/dashboard/settings/connect", false],
        ]);
    });
});

/**
 * A cold launch straight into a settings URL, or the error screen's deep link, pushed
 * nothing. There is no entry beneath to pop, and popping anyway leaves the app.
 */
describe("SettingsNavigation with nothing beneath it", () => {
    it("replaces rather than popping on the way out", async () => {
        await nav(true).exit("/dashboard/settings/connect");

        expect(pop).not.toHaveBeenCalled();
        expect(moves()).toEqual([["/dashboard", true]]);
    });

    it("replaces rather than popping on the way up", async () => {
        await nav(true).up("/dashboard/settings/connect");

        expect(pop).not.toHaveBeenCalled();
        expect(moves()).toEqual([["/dashboard/settings", true]]);
    });

    /**
     * The system back pops without telling this class. A counter would drift and
     * over-pop later; what is tracked instead is only whether the ancestors are ours,
     * with the depth read fresh from the path each time.
     */
    it("still pops correctly after the system back has moved us", async () => {
        const settings = nav(true);
        await settings.open();
        await settings.select("audio");
        pop.mockClear();
        navigate.mockClear();

        // The user pressed the system back: the pane entry is gone and we are on the
        // list. Nothing called into this class to say so.
        await settings.exit("/dashboard/settings");

        expect(pop).toHaveBeenCalledWith(-1);
        expect(navigate).not.toHaveBeenCalled();
    });
});
