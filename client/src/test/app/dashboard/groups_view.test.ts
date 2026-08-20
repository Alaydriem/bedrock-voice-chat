import { get } from "svelte/store";
import { describe, expect, it } from "vitest";
import { PlayerHue } from "$radial/core/sources/PlayerHue";
import { GroupsView } from "../../../js/app/dashboard/GroupsView";
import type { Channel } from "../../../js/bindings/Channel";

function channel(id: string, name: string, players: string[], creator = "Alaydriem"): Channel {
    return { id, name, players, creator } as Channel;
}

describe("GroupsView", () => {
    // The cluster's whole job is saying who is in there, and its faces are 22px. Initials do
    // that; the block glyph the cards carry does not survive the size.
    it("takes a face's letters from word initials where there are words", () => {
        const view = new GroupsView();
        const rows = view.rows(
            [channel("raid", "Raid party", ["minecraft:Some Gamer", "minecraft:Sombra"])],
            null,
        );

        const members = get(rows)[0].members;
        expect(members.find((m) => m.gamertag === "Some Gamer")?.initials).toBe("SG");
        expect(members.find((m) => m.gamertag === "Sombra")?.initials).toBe("SO");
    });

    // One player is one colour everywhere. The card derives its hue from the lowercased CN, so
    // a face derived from anything else would give the same person two identities on one screen.
    it("hues a face from the same key a player's card uses", () => {
        const view = new GroupsView();
        const rows = view.rows([channel("raid", "Raid", ["minecraft:Petra"])], null);

        expect(get(rows)[0].members[0].hue).toBe(PlayerHue.of("minecraft:petra"));
    });

    it("marks the group this client is in", () => {
        const view = new GroupsView();
        const rows = view.rows(
            [channel("raid", "Raid party", []), channel("build", "Build", [])],
            "build",
        );

        expect(get(rows).map((r) => r.joined)).toEqual([false, true]);
    });

    // Channel events reach every client, including for groups this one is not in, which is what
    // makes any activity signal possible from outside.
    it("stirs a row on a join and stops on its own", () => {
        const view = new GroupsView();
        view.stir("raid");
        const rows = view.rows([channel("raid", "Raid party", [])], null);

        expect(get(rows)[0].stirring).toBe(true);
        expect(get(rows)[0].activeAt).not.toBeNull();
    });

    it("says nothing about activity for a group nothing has happened in", () => {
        const view = new GroupsView();
        const rows = view.rows([channel("quiet", "Quiet", [])], null);

        expect(get(rows)[0].activeAt).toBeNull();
        expect(get(rows)[0].stirring).toBe(false);
    });

    // Ownership, not membership, is what gates renaming and closing. Somebody coordinating
    // several groups is in at most one of them and still owns all of them, so gating the admin
    // actions on `joined` would lock them out of every group but the one they are sitting in.
    it("marks a group this client created even when it is not in it", () => {
        const view = new GroupsView();
        const rows = view.rows(
            [channel("mine", "Mine", [], "minecraft:Alaydriem"), channel("theirs", "Theirs", [], "minecraft:Petra")],
            null,
            "minecraft:Alaydriem",
        );

        expect(get(rows).map((r) => r.owned)).toEqual([true, false]);
        expect(get(rows).map((r) => r.joined)).toEqual([false, false]);
    });

    // The creator and this client's own name both come from a certificate Common Name, so the
    // game prefix is part of the answer. A comparison that ignored it would hand somebody the
    // rename and close buttons for a group created by their namesake under another prefix.
    it("does not own a group created by the same gamertag under another game prefix", () => {
        const view = new GroupsView();
        const rows = view.rows(
            [channel("theirs", "Theirs", [], "othergame:Alaydriem")],
            null,
            "minecraft:Alaydriem",
        );

        expect(get(rows)[0].owned).toBe(false);
    });

    // Without a name there is nobody to compare against, and defaulting to owned would offer
    // every client the close button on every group.
    it("owns nothing when this client's name is unknown", () => {
        const view = new GroupsView();
        const rows = view.rows([channel("mine", "Mine", [], "minecraft:Alaydriem")], null);

        expect(get(rows)[0].owned).toBe(false);
    });

    it("carries the CN form so a member's hue matches their card", () => {
        const view = new GroupsView();
        // Membership arrives in both forms depending on its path; the bare one has to be
        // promoted or the hue is derived from a different string than the card's.
        const rows = view.rows([channel("raid", "Raid", ["Petra"])], null);

        expect(get(rows)[0].members[0].name).toBe("minecraft:Petra");
    });
});

describe("GroupsView.since", () => {
    it("says nothing when no activity has been seen", () => {
        expect(GroupsView.since(null, Date.now())).toBe("");
    });

    it("reads in whole minutes once it is worth counting them", () => {
        const now = 10_000_000;
        expect(GroupsView.since(now - 30_000, now)).toBe("active just now");
        expect(GroupsView.since(now - 120_000, now)).toBe("active 2 min ago");
        expect(GroupsView.since(now - 7_200_000, now)).toBe("active 2 h ago");
    });
});
