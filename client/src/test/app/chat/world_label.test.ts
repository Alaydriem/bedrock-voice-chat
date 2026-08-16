import { describe, expect, test } from "vitest";
import { WorldLabel } from "../../../js/app/chat/WorldLabel";

describe("WorldLabel.isPlaceholder", () => {
    // The mod sends the world's uuid when the operator has not named it. A uuid is
    // unmistakably not a name, which is the whole reason it was chosen over a constant.
    test("a uuid is a placeholder", () => {
        expect(WorldLabel.isPlaceholder("21f7a2b4-3c5e-4a19-9f2d-8c7b6e5a4d3c")).toBe(true);
    });

    test("a uuid in upper case or without dashes is still a placeholder", () => {
        expect(WorldLabel.isPlaceholder("21F7A2B4-3C5E-4A19-9F2D-8C7B6E5A4D3C")).toBe(true);
        expect(WorldLabel.isPlaceholder("21f7a2b43c5e4a199f2d8c7b6e5a4d3c")).toBe(true);
    });

    // Paper and Fabric report the real level name, and the default is literally `world`. It
    // tells nobody anything, and several servers in one list all carry it.
    test("a default level name is a placeholder", () => {
        for (const name of ["world", "World", "Minecraft world", "Bedrock level", "  world  "]) {
            expect(WorldLabel.isPlaceholder(name), name).toBe(true);
        }
    });

    test("an empty name is a placeholder", () => {
        expect(WorldLabel.isPlaceholder("")).toBe(true);
        expect(WorldLabel.isPlaceholder("   ")).toBe(true);
    });

    test("a name somebody chose is not a placeholder", () => {
        expect(WorldLabel.isPlaceholder("Truly Bedrock SMP")).toBe(false);
        expect(WorldLabel.isPlaceholder("Hearthhold")).toBe(false);
    });

    // A derived hash on the proxy path is not a uuid and is not a level name, but it is also
    // not something to show a person.
    test("the proxy path's derived world id is a placeholder", () => {
        expect(WorldLabel.isPlaceholder("a3f9c1d84b2e7605")).toBe(true);
    });
});

describe("WorldLabel.resolve", () => {
    const uuid = "21f7a2b4-3c5e-4a19-9f2d-8c7b6e5a4d3c";

    test("a real name is used as it is", () => {
        expect(WorldLabel.resolve(uuid, "Truly Bedrock SMP", {})).toBe("Truly Bedrock SMP");
    });

    // What the reader chose in BVC Connect, learned on a previous session and kept.
    test("a placeholder falls back to the remembered association", () => {
        expect(WorldLabel.resolve(uuid, uuid, { [uuid]: "Truly Bedrock SMP" })).toBe(
            "Truly Bedrock SMP",
        );
    });

    // A remembered name never overrides one the operator actually set.
    test("a remembered association does not override a real name", () => {
        expect(WorldLabel.resolve(uuid, "Operator's Name", { [uuid]: "Stale" })).toBe(
            "Operator's Name",
        );
    });

    // Nothing known either way. The uuid is still better than a constant every world shares:
    // at least two unnamed worlds are visibly different.
    test("an unknown placeholder shows an unnamed world with a short id", () => {
        const label = WorldLabel.resolve(uuid, uuid, {});
        expect(label).toContain("21f7a2b4");
        expect(label).not.toBe(uuid);
    });

    test("two unknown worlds do not render identically", () => {
        const other = "99887766-5544-3322-1100-aabbccddeeff";
        expect(WorldLabel.resolve(uuid, uuid, {})).not.toBe(
            WorldLabel.resolve(other, other, {}),
        );
    });
});
