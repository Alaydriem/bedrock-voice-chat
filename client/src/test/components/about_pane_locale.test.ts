import { render, waitFor } from "@testing-library/svelte";
import { beforeEach, expect, it, vi } from "vitest";
import { invokeCalls, mockInvoke } from "../tauri";
import { I18n } from "../../lib/i18n";

vi.mock("@tauri-apps/plugin-os", () => ({ platform: () => "windows" }));

const { default: AboutPane } = await import("../../components/settings/panes/AboutPane.svelte");
const { default: BVCApp } = await import("../../js/app/BVCApp");

const RUSSIAN = {
    v: 1,
    locale: "ru",
    plural: ["one", "few", "many"],
    m: { Language: "Язык" },
};

function updates() {
    return { state: { subscribe: (run: (v: unknown) => void) => (run({ kind: "idle" }), () => {}) } };
}

function mount() {
    const host = document.createElement("div");
    document.body.append(host);
    render(AboutPane as never, { target: host, props: { updates: updates() } } as never);
    return { host, text: () => host.textContent ?? "" };
}

function picker(host: HTMLElement): HTMLSelectElement {
    const found = [...host.querySelectorAll("select")].find((element) =>
        [...element.options].some((option) => option.value === "auto"),
    );
    if (!found) throw new Error("locale picker not rendered");
    return found;
}

/**
 * The picker markup exists before the manager resolves, so waiting on the element alone
 * hands back a disabled control with only the follow-the-system option on it.
 */
async function readyPicker(host: HTMLElement): Promise<HTMLSelectElement> {
    await waitFor(() => {
        if (picker(host).disabled) throw new Error("locale manager has not resolved");
    });
    return picker(host);
}

beforeEach(() => {
    I18n.adopt(null);
    // The manager is cached on the class for the app's lifetime, so a test that left one
    // behind would hand the next test a picker wired to the previous test's IPC mock.
    (BVCApp as unknown as { localeManagerPromise: unknown }).localeManagerPromise = null;

    mockInvoke({
        i18n_locales: () => ["de", "en_XA", "ru"],
        i18n_load: () => RUSSIAN,
        get_app_info: () => ({ version: "1.0.0", tauri_version: "2", build: "test" }),
        get_telemetry: () => true,
        get_platform_id: () => "b6f1c0e2-4d33-4a71-9c58-2f0e9a71d55c",
        get_credential: () => null,
    });
});

it("lists every bundled locale plus the follow-the-system option", async () => {
    const { host } = mount();

    await waitFor(() => {
        const options = [...picker(host).options].map((option) => option.value);
        expect(options).toEqual(["auto", "de", "en_XA", "ru"]);
    });
});

it("renders the pane's own copy through the loaded pack", async () => {
    const { host, text } = mount();

    await readyPicker(host);
    await waitFor(() => expect(text()).toContain("Язык"));
});

it("stays disabled until the pack has resolved, so a choice cannot be lost", () => {
    const { host } = mount();

    expect(picker(host).disabled).toBe(true);
});

it("choosing a locale asks the backend for that pack", async () => {
    const { host } = mount();
    const select = await readyPicker(host);

    select.value = "de";
    select.dispatchEvent(new Event("change", { bubbles: true }));

    await waitFor(() => {
        expect(invokeCalls()).toContainEqual(
            expect.objectContaining({ cmd: "i18n_load", args: { requested: "de" } }),
        );
    });
});
