import { describe, expect, it } from "vitest";
import GameNameUtils from "../../../js/app/utils/GameNameUtils";

describe("GameNameUtils.canonical", () => {
    it("prefixes a bare name with the game it came from", () => {
        expect(GameNameUtils.canonical("Alaydriem", "minecraft")).toBe("minecraft:Alaydriem");
        expect(GameNameUtils.canonical("Alaydriem", "hytale")).toBe("hytale:Alaydriem");
    });

    // Normalising an already-normalised name must be safe, because a name can pass through
    // more than one boundary before it is used as a key.
    it("is idempotent", () => {
        const once = GameNameUtils.canonical("Alaydriem", "minecraft");
        expect(GameNameUtils.canonical(once, "minecraft")).toBe(once);
    });

    // A name that already names its game wins. Re-prefixing it with the caller's guess would
    // move a player from one game to another.
    it("keeps the game the name already declares", () => {
        expect(GameNameUtils.canonical("hytale:Alaydriem", "minecraft")).toBe("hytale:Alaydriem");
    });

    it("defaults to minecraft when the caller has no game", () => {
        expect(GameNameUtils.canonical("Alaydriem")).toBe("minecraft:Alaydriem");
    });

    // Xbox gamertags contain spaces. A canonical form that split on whitespace would turn one
    // player into two.
    it("keeps a name with spaces whole", () => {
        expect(GameNameUtils.canonical("Some Gamer", "minecraft")).toBe("minecraft:Some Gamer");
    });

    // An empty name is not a player. Returning "minecraft:" would create a key that matches
    // nobody and never expires.
    it("refuses to canonicalise an empty name", () => {
        expect(GameNameUtils.canonical("", "minecraft")).toBe("");
        expect(GameNameUtils.canonical("   ", "minecraft")).toBe("");
    });
});
