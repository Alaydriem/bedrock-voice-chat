import { get } from "svelte/store";
import { describe, expect, it } from "vitest";
import "../../tauri";

const { PlayerManager } = await import("../../../js/app/managers/PlayerManager");

describe("PlayerManager identity", () => {
    // The login response carries a bare gamertag while every roster key is the certificate's
    // `game:gamertag`. Held bare, this client did not recognise itself in its own roster and
    // drew a card for the person holding the webview — at a volume they could adjust.
    it("recognises itself when it was told a bare gamertag", () => {
        const manager = new PlayerManager("Alaydriem", "minecraft");
        manager.add("minecraft:Alaydriem");
        manager.add("minecraft:Petra");

        expect(get(manager.activePlayers).map((p) => p.name)).toEqual(["minecraft:Petra"]);
    });

    // Two games can carry the same gamertag, and they are two people. A self-comparison that
    // ignored the game would silently hide one of them from the roster.
    it("keeps the same gamertag from another game on the roster", () => {
        const manager = new PlayerManager("Alaydriem", "minecraft");
        manager.add("minecraft:Alaydriem");
        manager.add("hytale:Alaydriem");

        expect(get(manager.activePlayers).map((p) => p.name)).toEqual(["hytale:Alaydriem"]);
    });

    // The gamertag arrives a second time once credentials are settled, by a different route.
    it("composes a gamertag set after construction", () => {
        const manager = new PlayerManager("", "hytale");
        manager.setCurrentUser("Alaydriem");

        expect(manager.getCurrentUser()).toBe("hytale:Alaydriem");
    });

    // An empty name is not a player. Prefixing it would produce a key that matches nobody and
    // would exclude nobody from the roster either.
    it("holds no identity before it has been told who this is", () => {
        const manager = new PlayerManager("", "minecraft");

        expect(manager.getCurrentUser()).toBe("");
    });
});
