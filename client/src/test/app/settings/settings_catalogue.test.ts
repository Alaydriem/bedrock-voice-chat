import { describe, expect, it } from "vitest";
import { SettingsCatalogue } from "../../../js/app/settings/SettingsCatalogue";

describe("SettingsCatalogue", () => {
    it("opens on the account pane", () => {
        expect(SettingsCatalogue.fallback).toBe("account");
        expect(SettingsCatalogue.find("account", false)).not.toBeNull();
    });

    // Recording is not supported on mobile and there is no global shortcut to bind on a
    // phone. A pane that exists only to explain its own absence is a dead end in a list
    // somebody scans every time they open settings.
    it("drops the panes the mobile build does not have", () => {
        const ids = SettingsCatalogue.for(true).map((pane) => pane.id);
        expect(ids).not.toContain("recordings");
        expect(ids).not.toContain("keybinds");
        expect(ids).toContain("audio");
        expect(ids).toContain("library");
    });

    it("keeps every pane on desktop", () => {
        expect(SettingsCatalogue.for(false)).toHaveLength(10);
    });

    // A deep link to a pane this build does not have must not resolve, or the router
    // lands on a blank stage with a title above it.
    it("refuses to find a desktop-only pane on mobile", () => {
        expect(SettingsCatalogue.find("recordings", true)).toBeNull();
        expect(SettingsCatalogue.find("recordings", false)?.title).toBe("Recordings");
    });

    it("refuses to find a pane that does not exist at all", () => {
        expect(SettingsCatalogue.find("nonsense", false)).toBeNull();
    });

    it("has no empty groups on mobile", () => {
        for (const group of SettingsCatalogue.groups(true)) {
            expect(group.panes.length).toBeGreaterThan(0);
        }
    });

    it("keeps the Bedrock group on both builds", () => {
        for (const mobile of [true, false]) {
            const names = SettingsCatalogue.groups(mobile).map((group) => group.name);
            expect(names).toContain("Minecraft Bedrock");
        }
    });

    // Plates and a seven-column table both need more than the 760px row measure.
    it("marks the panes that need the wide measure", () => {
        const wide = SettingsCatalogue.for(false)
            .filter((pane) => pane.wide)
            .map((pane) => pane.id);
        expect(wide.sort()).toEqual(["library", "players", "proxy", "realms"]);
    });
});
