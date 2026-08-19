import { describe, expect, it } from "vitest";
import { ManagePlayersView } from "../../../js/app/settings/ManagePlayersView";
import type { AdminUserRow } from "../../../js/bindings/AdminUserRow";
import type { PermissionEntry } from "../../../js/bindings/PermissionEntry";

function user(overrides: Record<string, unknown> = {}): AdminUserRow {
    return {
        gamertag: "Bob",
        game: "minecraft",
        banished: false,
        connected: false,
        permissions: [],
        created_at: 1_753_732_440,
        ...overrides,
    } as unknown as AdminUserRow;
}

describe("ManagePlayersView.row", () => {
    // Keyed on game:gamertag, because two players in different games can share a gamertag
    // and an `{#each}` keyed on the bare name would collapse them into one row.
    it("keys a row on the canonical identity", () => {
        expect(ManagePlayersView.row(user()).key).toBe("minecraft:Bob");
    });

    it("carries the effective permissions through", () => {
        const row = ManagePlayersView.row(user({ permissions: ["admin"] }));
        expect(row.permissions).toEqual(["admin"]);
    });

    it("dates the row from the registration timestamp", () => {
        expect(ManagePlayersView.row(user()).added).not.toBe("—");
    });

    // Zero is what an unset column reads as, and 1970 is not a fact about this player.
    it("shows no date rather than the epoch", () => {
        expect(ManagePlayersView.row(user({ created_at: 0 })).added).toBe("—");
    });
});

describe("ManagePlayersView.status", () => {
    it("reports a connected player as online", () => {
        expect(ManagePlayersView.status(user({ connected: true }))).toBe("online");
    });

    it("reports a player with no connection as offline", () => {
        expect(ManagePlayersView.status(user())).toBe("offline");
    });

    // Banning closes the live session, so a banned row that still shows as connected is
    // reading a stale registry. Banned has to win, or the operator sees a ban that looks
    // like it did not take.
    it("reports a banned player as banned even while the registry still lists them", () => {
        expect(ManagePlayersView.status(user({ banished: true, connected: true }))).toBe("banned");
    });
});

describe("ManagePlayersView paging", () => {
    it("counts the pages the server's total needs", () => {
        expect(ManagePlayersView.pageCount(17, 8)).toBe(3);
    });

    it("keeps one page when there is nothing to show", () => {
        expect(ManagePlayersView.pageCount(0, 8)).toBe(1);
    });

    // Searching from the last page of an unfiltered list would otherwise ask for a page
    // past the end and render as empty rather than as "no matches".
    it("clamps a page past the end of the filtered set", () => {
        expect(ManagePlayersView.clampPage(9, 17, 8)).toBe(2);
    });

    it("clamps a negative page to the first", () => {
        expect(ManagePlayersView.clampPage(-3, 17, 8)).toBe(0);
    });
});

describe("ManagePlayersView.state", () => {
    const entries: readonly PermissionEntry[] = [
        { permission: "admin", effect: "allow" },
        { permission: "audio_delete", effect: "deny" },
    ] as unknown as readonly PermissionEntry[];

    it("reads an explicit allow", () => {
        expect(ManagePlayersView.state(entries, "admin")).toBe("allow");
    });

    it("reads an explicit deny", () => {
        expect(ManagePlayersView.state(entries, "audio_delete")).toBe("deny");
    });

    // No override means the server default decides, which is a third state and not a deny.
    // Rendering it as deny would make Default unreachable in the editor.
    it("reads no override as default", () => {
        expect(ManagePlayersView.state(entries, "audio_upload")).toBe("default");
    });
});

describe("ManagePlayersView failure copy", () => {
    // The server refuses a self-ban with 409. Saying "conflict" tells an operator nothing.
    it("explains a refused self-ban", () => {
        expect(ManagePlayersView.banFailure("conflict")).toBe("You cannot ban yourself.");
    });

    it("explains a duplicate whitelist entry", () => {
        expect(ManagePlayersView.addFailure("conflict")).toBe(
            "That player is already on the whitelist.",
        );
    });

    it("explains a refused self-demotion", () => {
        expect(ManagePlayersView.permissionFailure("conflict")).toBe(
            "You cannot remove your own admin permission.",
        );
    });

    it("names a lost permission the same way everywhere", () => {
        const lost = "You no longer hold the admin permission.";
        expect(ManagePlayersView.banFailure("forbidden")).toBe(lost);
        expect(ManagePlayersView.addFailure("forbidden")).toBe(lost);
        expect(ManagePlayersView.permissionFailure("forbidden")).toBe(lost);
    });
});

describe("ManagePlayersView.gameLabel", () => {
    it("capitalizes the game for display", () => {
        expect(ManagePlayersView.gameLabel("minecraft")).toBe("Minecraft");
    });

    it("names a game it does not know rather than rendering nothing", () => {
        expect(ManagePlayersView.gameLabel("hytale")).toBe("Hytale");
    });
});

describe("ManagePlayersView.blocks", () => {
    // The strip is positional: slot 0 is presence and each permission keeps its own slot on
    // every row, so a column of rows can be read down. A held permission colours its slot
    // and an unheld one dims it; dropping the slot would shift the ones after it.
    it("gives every row the same number of slots", () => {
        const held = ManagePlayersView.blocks(ManagePlayersView.row(user({ permissions: ["admin"] })));
        const none = ManagePlayersView.blocks(ManagePlayersView.row(user()));
        expect(held).toHaveLength(1 + ManagePlayersView.EDITABLE.length);
        expect(none).toHaveLength(held.length);
    });

    it("paints presence green when the player is connected", () => {
        const blocks = ManagePlayersView.blocks(ManagePlayersView.row(user({ connected: true })));
        expect(blocks[0].color).toBe("var(--color-rad-ok)");
    });

    it("paints presence grey when the player is offline", () => {
        const blocks = ManagePlayersView.blocks(ManagePlayersView.row(user()));
        expect(blocks[0].color).toBe("var(--color-rad-line-2)");
    });

    it("paints presence as a fault when the player is banned", () => {
        const blocks = ManagePlayersView.blocks(ManagePlayersView.row(user({ banished: true })));
        expect(blocks[0].color).toBe("var(--color-rad-fault)");
    });

    it("colours a held permission and dims one that is not held", () => {
        const blocks = ManagePlayersView.blocks(ManagePlayersView.row(user({ permissions: ["admin"] })));
        const admin = blocks.find((block) => block.label === "Administrator");
        const upload = blocks.find((block) => block.label === "Upload sounds");
        expect(admin?.color).toBe("var(--color-rad-brand-lift)");
        expect(upload?.color).toBe(ManagePlayersView.BLOCK_OFF);
    });
});

describe("ManagePlayersView.blocksLabel", () => {
    // Colour alone is not a fact anyone can read: the strip needs an accessible name, or the
    // row loses its state for a screen reader and for anyone who cannot separate the hues.
    it("names presence and every permission held", () => {
        const row = ManagePlayersView.row(user({ connected: true, permissions: ["admin"] }));
        expect(ManagePlayersView.blocksLabel(row)).toBe("Online · Administrator");
    });

    it("names presence alone when no permission is held", () => {
        expect(ManagePlayersView.blocksLabel(ManagePlayersView.row(user()))).toBe("Offline");
    });

    it("says banned rather than offline for a banned player", () => {
        const row = ManagePlayersView.row(user({ banished: true }));
        expect(ManagePlayersView.blocksLabel(row)).toBe("Banned");
    });
});

describe("ManagePlayersView.isSelf", () => {
    const me = { gamertag: "Bob", game: "minecraft" } as const;

    it("recognises the signed-in player", () => {
        expect(ManagePlayersView.isSelf(ManagePlayersView.row(user()), me)).toBe(true);
    });

    it("does not match a different player", () => {
        expect(ManagePlayersView.isSelf(ManagePlayersView.row(user({ gamertag: "Carol" })), me)).toBe(
            false,
        );
    });

    // Two players in different games can share a gamertag, and they are not the same person.
    it("does not match the same gamertag in another game", () => {
        expect(ManagePlayersView.isSelf(ManagePlayersView.row(user({ game: "hytale" })), me)).toBe(
            false,
        );
    });

    // Before introspect answers there is no identity, and nothing may be assumed to be self.
    it("matches nobody when the identity is unknown", () => {
        expect(ManagePlayersView.isSelf(ManagePlayersView.row(user()), null)).toBe(false);
    });
});
