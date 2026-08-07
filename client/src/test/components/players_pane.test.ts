import { fireEvent, render, waitFor } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCalls, mockInvoke } from "../tauri";

vi.mock("@tauri-apps/plugin-store", () => ({
    Store: {
        load: async () => ({
            get: async () => "https://bvc.example.com",
            set: async () => {},
            save: async () => {},
        }),
    },
}));

const { default: PlayersPane } = await import(
    "../../components/settings/panes/PlayersPane.svelte"
);

const NOW = Date.now();

function row(cn: string, gain: number, muted: boolean, lastSeen: number | null = NOW) {
    return {
        key: { server: "https://bvc.example.com", cn },
        settings: { gain, muted, last_seen: lastSeen },
    };
}

function mount() {
    const host = document.createElement("div");
    document.body.append(host);
    render(PlayersPane as never, { target: host } as never);
    return {
        host,
        text: () => host.textContent ?? "",
        button: (label: string) =>
            host.querySelector<HTMLButtonElement>(`[aria-label^="${label}"]`),
        slider: (name: string) =>
            host.querySelector<HTMLInputElement>(`[aria-label="Volume for ${name}"]`),
        segment: (label: string) =>
            [...host.querySelectorAll<HTMLButtonElement>(".rad-segmented button")].find(
                (b) => b.textContent?.trim() === label,
            ),
        search: () => host.querySelector<HTMLInputElement>('input[type="search"]'),
    };
}

describe("PlayersPane", () => {
    beforeEach(() => {
        document.body.innerHTML = "";
    });

    // The default segment is the short list, because proximity writes a row for everyone who
    // walks past and almost none of them carry a decision.
    it("hides an untouched player until Everyone is chosen", async () => {
        mockInvoke({
            player_settings_list: () => [
                row("minecraft:Plain", 1.0, false),
                row("minecraft:Quiet", 0.4, false),
            ],
        });
        const view = mount();

        await waitFor(() => expect(view.text()).toContain("Quiet"));
        expect(view.text()).not.toContain("Plain");

        await fireEvent.click(view.segment("Everyone")!);
        await waitFor(() => expect(view.text()).toContain("Plain"));
    });

    // "Why can't I hear them" is the question this pane answers, so the answer is never
    // below somebody who was simply seen more recently.
    it("sorts a muted player above a more recently seen one", async () => {
        mockInvoke({
            player_settings_list: () => [
                row("minecraft:Recent", 0.5, false, NOW),
                row("minecraft:Muted", 1.0, true, NOW - 3_600_000),
            ],
        });
        const view = mount();

        await waitFor(() => expect(view.text()).toContain("Muted"));
        const names = [...view.host.querySelectorAll(".rad-recent-row__name")].map(
            (n) => n.textContent,
        );
        expect(names).toEqual(["Muted", "Recent"]);
    });

    // A slider that still moves while the player is muted invites the user to set a level
    // that has no audible effect, and then wonder why.
    it("disables the slider while muted and says so in the readout", async () => {
        mockInvoke({ player_settings_list: () => [row("minecraft:Muted", 0.8, true)] });
        const view = mount();

        await waitFor(() => expect(view.slider("Muted")).toBeTruthy());
        expect(view.slider("Muted")!.disabled).toBe(true);
        expect(view.text()).toContain("muted");
        expect(view.text()).not.toContain("80%");
    });

    it("sends the canonical key, not the display name, when muting", async () => {
        // Turned down, so the row is visible under the default Adjusted segment.
        mockInvoke({
            player_settings_list: () => [row("minecraft:Alaydriem", 0.6, false)],
            player_settings_set_muted: () => null,
        });
        const view = mount();

        await waitFor(() => expect(view.button("Mute Alaydriem")).toBeTruthy());
        await fireEvent.click(view.button("Mute Alaydriem")!);

        await waitFor(() => {
            const call = invokeCalls().find((c) => c.cmd === "player_settings_set_muted");
            expect(call?.args).toMatchObject({ cn: "minecraft:Alaydriem", muted: true });
        });
    });

    // Reset is destructive and easy to hit by accident next to a list of names.
    it("asks before putting everyone back to normal", async () => {
        mockInvoke({
            player_settings_list: () => [row("minecraft:Quiet", 0.4, false)],
            player_settings_reset_all: () => null,
        });
        const view = mount();

        // Wait for the list, not the button: the reset card is static, so its text is present
        // before anything has loaded — and the button is disabled until a row carries a
        // decision, so clicking early would silently do nothing.
        await waitFor(() => expect(view.text()).toContain("Quiet"));
        const trigger = [...view.host.querySelectorAll<HTMLButtonElement>("button")].find((b) =>
            b.textContent?.includes("Reset everybody…"),
        );
        expect(trigger?.disabled).toBe(false);
        await fireEvent.click(trigger!);

        await waitFor(() => expect(view.text()).toContain("Reset everybody?"));
        expect(invokeCalls().some((c) => c.cmd === "player_settings_reset_all")).toBe(false);

        const confirm = [...view.host.querySelectorAll<HTMLButtonElement>(".rad-modal__actions button")].find(
            (b) => b.textContent?.trim() === "Reset",
        );
        await fireEvent.click(confirm!);
        await waitFor(() =>
            expect(invokeCalls().some((c) => c.cmd === "player_settings_reset_all")).toBe(true),
        );
    });

    it("says nothing has been changed when no row carries a decision", async () => {
        mockInvoke({ player_settings_list: () => [row("minecraft:Plain", 1.0, false)] });
        const view = mount();

        await waitFor(() => expect(view.text()).toContain("Nothing changed yet"));
    });

    it("says nobody is here at all when the store is empty", async () => {
        mockInvoke({ player_settings_list: () => [] });
        const view = mount();

        await waitFor(() => expect(view.text()).toContain("Nothing changed yet"));
        await fireEvent.click(view.segment("Everyone")!);
        await waitFor(() => expect(view.text()).toContain("Nobody yet"));
    });

    it("says nobody matches when a search comes up empty", async () => {
        mockInvoke({ player_settings_list: () => [row("minecraft:Quiet", 0.4, false)] });
        const view = mount();

        await waitFor(() => expect(view.text()).toContain("Quiet"));
        await fireEvent.input(view.search()!, { target: { value: "zzz" } });

        await waitFor(() => expect(view.text()).toContain("Nobody matches that"));
    });
});
