import { render, waitFor } from "@testing-library/svelte";
import { beforeEach, expect, it, vi } from "vitest";
import { invokeCalls, mockInvoke } from "../tauri";
import { I18n } from "../../lib/i18n";

vi.mock("@tauri-apps/plugin-os", () => ({ platform: () => "windows" }));

const { default: AboutPane } = await import("../../components/settings/panes/AboutPane.svelte");
const { default: BVCApp } = await import("../../js/app/BVCApp");

const CURRENT = "b6f1c0e2-4d33-4a71-9c58-2f0e9a71d55c";
const REPLACEMENT = "9f3c1d84-2b57-4c0a-9e11-6d4a7b28e0f5";

function updates() {
    return { state: { subscribe: (run: (v: unknown) => void) => (run({ kind: "idle" }), () => {}) } };
}

function mount() {
    const host = document.createElement("div");
    document.body.append(host);
    render(AboutPane as never, { target: host, props: { updates: updates() } } as never);
    return { host, text: () => host.textContent ?? "" };
}

function refreshButton(host: HTMLElement): HTMLButtonElement {
    const field = host.querySelector(".rad-idfield");
    const found = field?.querySelector<HTMLButtonElement>(".rad-btn");
    if (!found) throw new Error("the platform ID row has no refresh button");
    return found;
}

function value(host: HTMLElement): string {
    return host.querySelector(".rad-idfield__value")?.textContent?.trim() ?? "";
}

beforeEach(() => {
    I18n.adopt(null);
    (BVCApp as unknown as { localeManagerPromise: unknown }).localeManagerPromise = null;

    mockInvoke({
        i18n_locales: () => [],
        i18n_load: () => null,
        get_app_info: () => ({ app_version: "1.0.0", protocol_version: "3.0.0" }),
        get_telemetry: () => true,
        get_platform_id: () => CURRENT,
        refresh_platform_id: () => REPLACEMENT,
    });
});

// It used to take three presses on the release type. Support asks every reporter for
// this value, and a control nobody can find is one support has to talk them through.
it("shows the platform ID without any gesture to reveal it", async () => {
    const { host } = mount();

    await waitFor(() => expect(value(host)).toBe(CURRENT));
});

it("refreshing shows the identity the backend minted, not one the screen invented", async () => {
    const { host } = mount();
    await waitFor(() => expect(value(host)).toBe(CURRENT));

    refreshButton(host).click();

    await waitFor(() => expect(value(host)).toBe(REPLACEMENT));
    expect(invokeCalls().map((call) => call.cmd)).toContain("refresh_platform_id");
});

// A refresh that did not land must not leave a new id on screen: the session is still
// reporting under the old one, and that is the value support needs.
it("keeps the current ID and says why when the refresh fails", async () => {
    mockInvoke({
        i18n_locales: () => [],
        i18n_load: () => null,
        get_app_info: () => ({ app_version: "1.0.0", protocol_version: "3.0.0" }),
        get_telemetry: () => true,
        get_platform_id: () => CURRENT,
        refresh_platform_id: () => {
            throw new Error("Failed to save platform ID");
        },
    });
    const { host, text } = mount();
    await waitFor(() => expect(value(host)).toBe(CURRENT));

    refreshButton(host).click();

    await waitFor(() => expect(text()).toContain("Failed to save platform ID"));
    expect(value(host)).toBe(CURRENT);
});
