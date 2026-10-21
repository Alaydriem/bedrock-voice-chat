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
    // One kind of group: protection is a switch in the editor, not a second create button.
    it("offers only New group", () => {
        const host = mount({ protectedJoined: false });

        expect(host.querySelector(".rad-new-group--protected")).toBeNull();
        expect(host.textContent).not.toContain("New protected group");
    });

    // The safety code is on demand, behind a button; no banner and no always-on code.
    it("shows no banner and no code until asked", () => {
        const host = mount({ protectedJoined: true, safetyCode: "12345 67890" });

        expect(host.querySelector(".rad-protected-banner")).toBeNull();
        expect(host.querySelector(".rad-safety-code")).toBeNull();
        expect(host.textContent).not.toContain("12345 67890");
    });
});
