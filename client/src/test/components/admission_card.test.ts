import { render } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";

const { default: AdmissionCard } = await import("../../components/dashboard/AdmissionCard.svelte");

const CHECK = "cd".repeat(16);

function mount(props: Record<string, unknown>) {
    const host = document.createElement("div");
    document.body.append(host);
    const handlers = {
        oncheck: vi.fn(),
        ondecide: vi.fn(),
        onshow: vi.fn(),
        onlater: vi.fn(),
        ondismiss: vi.fn(),
    };
    render(
        AdmissionCard as never,
        {
            target: host,
            props: {
                joinerName: "minecraft:Steve",
                groupName: "Night Raid",
                phase: "request",
                expanded: false,
                code: null,
                checkId: null,
                otherMember: null,
                secondsLeft: null,
                canCheck: true,
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

describe("AdmissionCard", () => {
    it("asks whether to admit, with Not now and Check code", () => {
        const { host, oncheck, onlater } = mount({ phase: "request" });

        expect(host.textContent).toContain("Admit Steve to Night Raid?");
        button(host, "Check code")?.click();
        button(host, "Not now")?.click();
        expect(oncheck).toHaveBeenCalledOnce();
        expect(onlater).toHaveBeenCalledOnce();
    });

    it("disables Check code while three checks are open", () => {
        const { host } = mount({ phase: "request", canCheck: false });

        expect(button(host, "Check code")?.disabled).toBe(true);
    });

    it("shows the code with Deny and Admit Steve only when expanded", () => {
        const { host, ondecide } = mount({
            phase: "code",
            expanded: true,
            code: "4821 0937",
            checkId: CHECK,
        });

        expect(host.textContent).toContain("Check Steve's code");
        expect(host.textContent).toContain(
            "Steve reads you this code. Admit only if it is Steve and the code matches.",
        );
        expect(host.querySelector(".rad-digits")?.textContent).toBe("4821 0937");
        button(host, "Admit Steve")?.click();
        expect(ondecide).toHaveBeenCalledWith(CHECK, true);
        button(host, "Deny")?.click();
        expect(ondecide).toHaveBeenCalledWith(CHECK, false);
    });

    it("a ready code that is not expanded offers only Show code", () => {
        const { host, onshow } = mount({ phase: "code", expanded: false, code: "4821 0937", checkId: CHECK });

        expect(host.textContent).toContain("Steve's code is ready");
        expect(host.querySelector(".rad-digits")).toBeNull();
        expect(button(host, "Admit Steve")).toBeUndefined();
        expect(button(host, "Deny")).toBeUndefined();
        button(host, "Show code")?.click();
        expect(onshow).toHaveBeenCalledOnce();
    });

    it("says another member is checking, with no buttons", () => {
        const { host } = mount({ phase: "checking_elsewhere", otherMember: "minecraft:EL" });

        expect(host.textContent).toContain("EL is checking Steve");
        expect(host.querySelectorAll("button").length).toBe(0);
    });

    it("a failed check says so and can be dismissed", () => {
        const { host, ondismiss } = mount({ phase: "failed" });

        expect(host.textContent).toContain("Check failed");
        expect(host.textContent).toContain("Someone may be interfering.");
        button(host, "Dismiss")?.click();
        expect(ondismiss).toHaveBeenCalledOnce();
    });

    it("an admitted joiner reads as joined", () => {
        const { host } = mount({ phase: "admitted" });

        expect(host.textContent).toContain("Steve joined Night Raid");
    });
});
