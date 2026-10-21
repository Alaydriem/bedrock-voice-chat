import { render } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";

const { default: JoinProtectedDialog } = await import(
    "../../components/dashboard/JoinProtectedDialog.svelte"
);

function mount(props: Record<string, unknown>) {
    const host = document.createElement("div");
    document.body.append(host);
    const handlers = { onask: vi.fn(), onconfirm: vi.fn(), oncancel: vi.fn() };
    render(
        JoinProtectedDialog as never,
        {
            target: host,
            props: {
                name: "Night Raid",
                phase: "ask",
                code: null,
                checkId: null,
                checker: null,
                ...handlers,
                ...props,
            },
        } as never,
    );
    return { host, ...handlers };
}

function button(host: HTMLElement, label: string): HTMLButtonElement | undefined {
    return [...host.querySelectorAll<HTMLButtonElement>("button")].find(
        (b) => b.textContent?.trim() === label,
    );
}

describe("JoinProtectedDialog", () => {
    it("asks first, with the group's name", () => {
        const { host, onask } = mount({ phase: "ask" });

        expect(host.textContent).toContain("Join Night Raid");
        expect(host.textContent).toContain("A member must admit you.");
        button(host, "Ask to join")?.click();
        expect(onask).toHaveBeenCalledOnce();
    });

    it("shows the code to read out, by the checking member's name", () => {
        const { host, onconfirm } = mount({
            phase: "code",
            code: "4821 0937",
            checkId: "ab".repeat(16),
            checker: "minecraft:AL",
        });

        expect(host.textContent).toContain("Read this code to AL");
        expect(host.querySelector(".rad-digits")?.textContent).toBe("4821 0937");
        expect(host.textContent).toContain("In person, by phone or on Discord. Not in game chat.");
        button(host, "It matches")?.click();
        expect(onconfirm).toHaveBeenCalledWith("ab".repeat(16), true);
        button(host, "No match")?.click();
        expect(onconfirm).toHaveBeenCalledWith("ab".repeat(16), false);
    });

    it("says the codes did not match and offers to ask again", () => {
        const { host, onask } = mount({ phase: "mismatch" });

        expect(host.textContent).toContain("The codes did not match");
        button(host, "Ask again")?.click();
        expect(onask).toHaveBeenCalledOnce();
    });

    it("names the member who did not admit", () => {
        const { host } = mount({ phase: "refused", checker: "minecraft:AL" });

        expect(host.textContent).toContain("AL did not admit you");
    });

    it("has a line for each other ending", () => {
        expect(mount({ phase: "expired" }).host.textContent).toContain("No member answered");
        expect(mount({ phase: "no_admitter" }).host.textContent).toContain("No member is online");
        expect(mount({ phase: "joining" }).host.textContent).toContain("Joining Night Raid");
    });
});
