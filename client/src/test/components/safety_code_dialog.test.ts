import { render } from "@testing-library/svelte";
import { flushSync } from "svelte";
import { describe, expect, it } from "vitest";

const { default: ProtectedLine } = await import("../../components/dashboard/ProtectedLine.svelte");

function mount(safetyCode: string | null) {
    const host = document.createElement("div");
    document.body.append(host);
    render(
        ProtectedLine as never,
        {
            target: host,
            props: { safetyCode, groupName: "Night Raid", memberCount: 4 },
        } as never,
    );
    return host;
}

describe("ProtectedLine and SafetyCodeDialog", () => {
    it("says who can hear and keeps the code hidden until asked", () => {
        const host = mount("20418 77305 61942 08867 43150 95521");

        expect(host.textContent).toContain("Players nearby cannot hear you.");
        expect(host.textContent).not.toContain("20418");
        expect(document.querySelector(".rad-code__grid")).toBeNull();
    });

    it("shows six cells of the code once Safety code is pressed", () => {
        const host = mount("20418 77305 61942 08867 43150 95521");

        host.querySelector<HTMLButtonElement>(".rad-sealed-line__code")?.click();
        flushSync();

        const cells = document.querySelectorAll(".rad-code__grid span");
        expect(cells.length).toBe(6);
        expect(cells[0]?.textContent).toBe("20418");
        expect(document.body.textContent).toContain(
            "Read it out to the others. If yours is different, leave the group.",
        );
    });
});
