import { render } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import type { GroupRowView } from "../../js/app/dashboard/GroupRowView";

const { default: GroupsPanel } = await import("../../components/dashboard/GroupsPanel.svelte");

function mount(props: Record<string, unknown>) {
    const host = document.createElement("div");
    document.body.append(host);
    render(
        GroupsPanel as never,
        {
            target: host,
            props: {
                groups: [] as GroupRowView[],
                now: Date.now(),
                onjoin: vi.fn(),
                oncreate: vi.fn(),
                ...props,
            },
        } as never,
    );
    return host;
}

describe("GroupsPanel protected groups", () => {
    // An older server 404s the protected create, so offering it there is offering a failure.
    it("protected_option_hidden_when_server_lacks_support", () => {
        const host = mount({ protectedAvailable: false, oncreateprotected: vi.fn() });

        expect(host.querySelector(".rad-new-group--protected")).toBeNull();
    });

    it("offers a protected group when the server supports it", () => {
        const oncreateprotected = vi.fn();
        const host = mount({ protectedAvailable: true, oncreateprotected });

        const button = host.querySelector<HTMLButtonElement>(".rad-new-group--protected");
        button?.click();

        expect(oncreateprotected).toHaveBeenCalledOnce();
    });

    it("says nearby players cannot hear a member of a protected group", () => {
        const host = mount({ protectedAvailable: true, protectedJoined: true, safetyCode: "12345 67890" });

        expect(host.textContent).toContain("Players nearby cannot hear you.");
        expect(host.textContent).toContain("Compare now");
        expect(host.textContent).toContain("12345 67890");
    });
});
