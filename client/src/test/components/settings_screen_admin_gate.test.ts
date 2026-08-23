import { render, waitFor } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { mockInvoke } from "../tauri";

vi.mock("@tauri-apps/plugin-os", () => ({ platform: () => "windows" }));

const { default: SettingsScreen } = await import(
    "../../components/settings/SettingsScreen.svelte"
);

function mount(pane: string) {
    const frame = document.createElement("div");
    frame.className = "rad-frame rad-frame--fluid";
    document.body.append(frame);

    render(SettingsScreen as never, {
        target: frame,
        props: {
            pane,
            level: "detail",
            onnavigate: vi.fn(),
            onclose: vi.fn(),
            onback: vi.fn(),
        },
    } as never);

    return {
        title: () => frame.querySelector<HTMLElement>(".rad-dash-top__server")?.textContent?.trim(),
        navLabels: () =>
            [...frame.querySelectorAll<HTMLElement>(".rad-panel__body .rad-nav-item")].map((item) =>
                item.textContent?.trim(),
            ),
    };
}

describe("SettingsScreen admin gate", () => {
    beforeEach(() => {
        document.body.innerHTML = "";
    });

    // The route resolves the path without knowing the permissions, so this render is the
    // gate: an admin gets the pane, and it is reachable by navigation and by deep link.
    it("renders the admin pane for a viewer holding admin", async () => {
        mockInvoke({
            get_credential: () => JSON.stringify({ allowed: ["admin"] }),
            api_introspect: () => ({ permissions: ["admin"] }),
            admin_list_users: () => ({ items: [], total: 0, page: 0, page_size: 8 }),
        });

        const view = mount("manage-players");

        await waitFor(() => expect(view.navLabels()).toContain("Manage Players"));
        expect(view.title()).toBe("Manage Players");
    });

    // And a viewer without it gets the fallback pane instead of an empty stage — the deep
    // link is stopped here rather than at the router.
    it("falls back to the first pane for a viewer without admin", async () => {
        mockInvoke({
            get_credential: () => JSON.stringify({ allowed: [] }),
            api_introspect: () => ({ permissions: [] }),
        });

        const view = mount("manage-players");

        await waitFor(() => expect(view.title()).toBe("Account"));
        expect(view.navLabels()).not.toContain("Manage Players");
    });

    it("names the per-player audio pane so it does not read as the roster", async () => {
        mockInvoke({
            get_credential: () => JSON.stringify({ allowed: [] }),
            api_introspect: () => ({ permissions: [] }),
        });

        const view = mount("account");

        await waitFor(() => expect(view.navLabels()).toContain("Player audio levels"));
    });
});
