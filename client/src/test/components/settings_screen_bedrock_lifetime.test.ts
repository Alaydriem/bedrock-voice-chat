import { render, waitFor } from "@testing-library/svelte";
import { get } from "svelte/store";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCalls, mockInvoke } from "../tauri";
import type { BedrockLogEntry } from "../../js/bindings/BedrockLogEntry";

vi.mock("@tauri-apps/plugin-os", () => ({ platform: () => "windows" }));

// The manager subscribes to `bedrock-log` on the webview. Driving that event is how these
// tests observe the buffer, which is the thing bug A1 was about losing.
let emit: ((entry: BedrockLogEntry) => void) | null = null;

vi.mock("@tauri-apps/api/webviewWindow", () => ({
    getCurrentWebviewWindow: () => ({
        listen: async (name: string, handler: (event: { payload: BedrockLogEntry }) => void) => {
            if (name === "bedrock-log") {
                emit = (entry) => handler({ payload: entry });
            }
            return () => {
                if (name === "bedrock-log") emit = null;
            };
        },
        emit: async () => {},
        label: "main",
    }),
}));

const { default: SettingsScreen } = await import(
    "../../components/settings/SettingsScreen.svelte"
);
const { BedrockManagerHolder } = await import("../../js/app/shell/BedrockManagerHolder");
const { BEDROCK_MANAGER_KEY } = await import("../../js/app/shell/BedrockManagerContext");

function mount(context?: Map<symbol, unknown>) {
    const frame = document.createElement("div");
    frame.className = "rad-frame rad-frame--fluid";
    document.body.append(frame);

    return render(SettingsScreen, {
        target: frame,
        props: {
            pane: "connect",
            level: "detail",
            onnavigate: () => {},
            onclose: () => {},
            onback: () => {},
        },
        ...(context ? { context } : {}),
    });
}

function settled(): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, 0));
}

let clock = 1_700_000_000_000;

beforeEach(() => {
    emit = null;
    clock = 1_700_000_000_000;
    vi.spyOn(Date, "now").mockImplementation(() => clock);
    mockInvoke({
        api_get_config: () => ({
            config: {
                status: "Ok",
                client_id: "cid",
                protocol_version: "3.0.0",
                quic_port: 0,
                quic_ports: [1],
                bedrock: { enabled: true, servers: [] },
            },
            client_version: "3.0.0",
            compatible: true,
            client_too_old: false,
        }),
        bedrock_get_status: () => ({
            proxy_running: false,
            realms_running: false,
            xbox_authenticated: false,
        }),
        bedrock_restore_auth: () => false,
        bedrock_list_protocol_versions: () => [],
        api_get_permissions: () => [],
    });
});

/**
 * The connection log lives on the Bedrock manager. The screen used to build and destroy that
 * manager itself, so closing settings threw the log away and reopening showed an empty pane
 * while the proxy was still running.
 */
describe("SettingsScreen Bedrock manager lifetime", () => {
    it("keeps the log the proxy already wrote across a close and reopen", async () => {
        const holder = new BedrockManagerHolder();
        const context = new Map<symbol, unknown>([[BEDROCK_MANAGER_KEY, holder]]);

        const first = mount(context);
        await waitFor(() => expect(emit).not.toBeNull());
        emit?.({
            timestamp_ms: 0n,
            level: "INFO",
            target: "bedrock_protocol",
            message: "proxy listening",
        });
        first.unmount();
        await settled();

        const second = mount(context);
        await settled();

        const lines = get(holder.get().realmsLogs);
        expect(
            lines.map((line) => line.message),
            "closing settings must not empty a log the dashboard still owns",
        ).toContain("proxy listening");

        second.unmount();
        holder.destroy();
    });

    // The error screen reaches settings with no dashboard behind it. That route has no
    // session to outlive, so the screen owns its holder — and must release it, or every
    // visit leaves another capability check refreshing on window focus.
    it("releases the holder it built when the dashboard published none", async () => {
        const view = mount();
        await waitFor(() =>
            expect(invokeCalls().some((call) => call.cmd === "api_get_config")).toBe(true),
        );

        view.unmount();
        await settled();
        mockInvoke({ api_get_config: () => ({}) });
        // Past the focus refresh throttle, or a live listener would decline the event for
        // its own reasons and this would prove nothing.
        clock += 60_000;
        window.dispatchEvent(new Event("focus"));
        await settled();

        expect(invokeCalls().filter((call) => call.cmd === "api_get_config")).toHaveLength(0);
    });
});
