import { render, waitFor } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { mockInvoke } from "../tauri";

let platformName = "windows";
vi.mock("@tauri-apps/plugin-os", () => ({ platform: () => platformName }));

vi.mock("@tauri-apps/plugin-store", () => ({
    Store: {
        load: async () => ({
            get: async () => "https://bvc.example.com",
            set: async () => {},
            save: async () => {},
        }),
    },
}));

const { default: AccountPane } = await import(
    "../../components/settings/panes/AccountPane.svelte"
);
const { default: AudioPane } = await import("../../components/settings/panes/AudioPane.svelte");

function mount(component: unknown, props: Record<string, unknown> = {}) {
    const host = document.createElement("div");
    document.body.append(host);
    render(component as never, { target: host, props } as never);
    return { host, text: () => host.textContent ?? "" };
}

function discord(overrides: Record<string, unknown> = {}) {
    return {
        configured: true,
        linked: false,
        role_count: 0,
        last_synced: null,
        expired: false,
        ...overrides,
    };
}

function account(credentials: Record<string, string>, link = discord()) {
    mockInvoke({
        get_credential: (args: never) => {
            const { key } = args as unknown as { key: string };
            const value = credentials[key];
            if (value === undefined) throw new Error("no credential");
            return value;
        },
        get_app_info: () => ({
            app_version: "1.0.0-beta.8",
            protocol_version: "2.1.0",
            build_commit: "8fa727ab",
            build_variant: "stable",
            build_number: "1",
        }),
        get_telemetry: () => true,
        discord_status: () => link,
    });
}

beforeEach(() => {
    mockInvoke({});
    platformName = "windows";
});

describe("AccountPane", () => {
    it("states how you signed in once, not three times", async () => {
        account({ gamertag: "Alaydriem", gamerpic: "" });
        const view = mount(AccountPane, { onsignout: () => {} });
        await waitFor(() => expect(view.text()).toContain("SIGNED IN WITH XBOX LIVE"));
        expect(view.text()).not.toContain("Signed in");
        expect(view.text()).not.toContain("HOW YOU SIGN IN");
    });

    // Java linking opens a native OAuth window that only the desktop app has, so the button
    // would report a failure every time. The row says where the linking happens instead.
    it("does not offer Java linking on mobile", async () => {
        account({ gamertag: "Al", gamerpic: "" });
        platformName = "android";
        const view = mount(AccountPane, { onsignout: () => {} });
        await waitFor(() => expect(view.text()).toContain("LINK THIS FROM THE DESKTOP APP"));
        const link = [...view.host.querySelectorAll<HTMLButtonElement>(".rad-btn")].filter(
            (b) => b.textContent?.trim() === "Link",
        );
        expect(link).toHaveLength(0);
    });

    it("offers the link on an account that is not linked", async () => {
        account({ gamertag: "Alaydriem", gamerpic: "" });
        const view = mount(AccountPane, { onsignout: () => {} });
        await waitFor(() => expect(view.text()).toContain("Alaydriem"));
        expect(view.text()).toContain("NOT LINKED");
    });

    it("names the Java account once it is linked", async () => {
        account({ gamertag: "Alaydriem", gamerpic: "", minecraft_username: "Alaydriem" });
        const view = mount(AccountPane, { onsignout: () => {} });
        await waitFor(() => expect(view.text()).toContain("LINKED AS ALAYDRIEM"));
    });

    // An expired link is not the same as one that was never made: the roles it granted
    // are gone, and the fix is to link again rather than to wonder why nothing happened.
    it("distinguishes an expired Discord link from an absent one", async () => {
        account({ gamertag: "Al", gamerpic: "" }, discord({ configured: true, linked: true, expired: true }));
        const view = mount(AccountPane, { onsignout: () => {} });
        await waitFor(() => expect(view.text()).toContain("LINK EXPIRED"));
    });

    // `configured` reports whether this build had Discord credentials compiled in. That is a
    // fact about our deployment and a reader can do nothing about it, so the row goes rather
    // than explaining itself.
    it("hides Discord entirely when the build has no credentials", async () => {
        account({ gamertag: "Al", gamerpic: "" }, discord({ configured: false }));
        const view = mount(AccountPane, { onsignout: () => {} });
        await waitFor(() => expect(view.text()).toContain("Minecraft Java"));
        expect(view.text()).not.toContain("Discord");
        expect(view.text()).not.toContain("NOT AVAILABLE");
    });

    it("offers Discord when the build can link it", async () => {
        account({ gamertag: "Al", gamerpic: "" }, discord({ configured: true }));
        const view = mount(AccountPane, { onsignout: () => {} });
        await waitFor(() => expect(view.text()).toContain("Discord"));

        const link = [...view.host.querySelectorAll<HTMLButtonElement>(".rad-btn")].filter(
            (b) => b.textContent?.trim() === "Link",
        );
        expect(link.every((b) => !b.disabled)).toBe(true);
    });

    it("hands signing out back to whoever owns the session", async () => {
        account({ gamertag: "Al", gamerpic: "" });
        const onsignout = vi.fn();
        const view = mount(AccountPane, { onsignout });
        await waitFor(() => expect(view.text()).toContain("Sign out of this server"));
        view.host.querySelector<HTMLElement>(".rad-btn--danger")?.click();
        expect(onsignout).toHaveBeenCalledTimes(1);
    });
});

describe("AudioPane", () => {
    // Android and iOS route audio themselves. An app-level picker there is a control
    // that either lies or fights the system.
    it("states that the system chose the device on mobile", async () => {
        const view = mount(AudioPane, { mobile: true });
        await waitFor(() => expect(view.text()).toContain("Chosen by the system"));
    });

    // The gate and the panning slider are the same on both, because both are ours.
    it("keeps the voice controls on mobile", async () => {
        const view = mount(AudioPane, { mobile: true });
        await waitFor(() => expect(view.text()).toContain("Spatial panning"));
        expect(view.text()).toContain("Voice mode");
        expect(view.text()).toContain("Noise gate");
    });
});
