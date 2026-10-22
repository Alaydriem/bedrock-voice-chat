import { render } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";

const { default: LeavePrivateGroupDialog } = await import(
    "../../components/dashboard/LeavePrivateGroupDialog.svelte"
);

function mount(closes: boolean) {
    const host = document.createElement("div");
    document.body.append(host);
    const handlers = { onconfirm: vi.fn(), oncancel: vi.fn() };
    render(
        LeavePrivateGroupDialog as never,
        { target: host, props: { name: "Night Raid", closes, ...handlers } } as never,
    );
    return { host, ...handlers };
}

function button(host: HTMLElement, label: string): HTMLButtonElement | undefined {
    return [...host.querySelectorAll<HTMLButtonElement>("button")].find(
        (b) => b.textContent?.trim() === label,
    );
}

describe("LeavePrivateGroupDialog", () => {
    it("asks to leave, and says a member must admit you to come back", () => {
        const { host, onconfirm } = mount(false);

        expect(host.textContent).toContain("Leave Night Raid?");
        expect(host.textContent).toContain("To come back, a member must admit you.");
        button(host, "Leave")?.click();
        expect(onconfirm).toHaveBeenCalledOnce();
    });

    it("says leaving closes the group when you are the last member", () => {
        const { host, onconfirm } = mount(true);

        expect(host.textContent).toContain("Close Night Raid?");
        expect(host.textContent).toContain("You are the last member. Leaving closes the group.");
        button(host, "Leave and close")?.click();
        expect(onconfirm).toHaveBeenCalledOnce();
    });

    it("leaves nothing on Cancel or Escape", () => {
        const { host, onconfirm, oncancel } = mount(false);

        button(host, "Cancel")?.click();
        window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
        expect(oncancel).toHaveBeenCalledTimes(2);
        expect(onconfirm).not.toHaveBeenCalled();
    });
});
