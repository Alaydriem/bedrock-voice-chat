import { beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCalls, mockInvoke } from "../../tauri";

vi.mock("@tauri-apps/plugin-os", () => ({ platform: () => "windows" }));

// Which webview listeners are attached right now. The shared mock in `src/test/tauri.ts`
// hands back an unlisten that records nothing, and whether a listener outlived its owner is
// exactly what one of these tests is about.
const attached = new Set<string>();

vi.mock("@tauri-apps/api/webviewWindow", () => ({
    getCurrentWebviewWindow: () => ({
        listen: async (name: string) => {
            attached.add(name);
            return () => attached.delete(name);
        },
        emit: async () => {},
        label: "main",
    }),
}));

function logListenerAttached(): boolean {
    return attached.has("bedrock-log");
}

const { BedrockManagerHolder } = await import(
    "../../../js/app/shell/BedrockManagerHolder"
);

function config() {
    return {
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
    };
}

function settled(): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, 0));
}

/** Waits until the capability check has actually run, so a later assertion is not vacuous. */
async function checked(): Promise<void> {
    for (let i = 0; i < 50; i += 1) {
        if (invokeCalls().some((call) => call.cmd === "api_get_config")) return;
        await settled();
    }
    throw new Error("the capability check never ran");
}

// The focus refresh is throttled against the wall clock, so the tests move the clock
// rather than wait on it.
let clock = 1_700_000_000_000;

beforeEach(() => {
    clock = 1_700_000_000_000;
    attached.clear();
    vi.spyOn(Date, "now").mockImplementation(() => clock);
    mockInvoke({
        api_get_config: () => config(),
        bedrock_get_status: () => ({
            proxy_running: false,
            realms_running: false,
            xbox_authenticated: false,
        }),
        bedrock_restore_auth: () => false,
    });
});

describe("Bedrock manager holder", () => {
    // Nothing is built until the Connect pane asks. The manager restores the Microsoft
    // session and reads /api/config on construction, and paying for that on every dashboard
    // mount would put it in the launch path of a session that never opens settings.
    it("builds nothing until asked", () => {
        const holder = new BedrockManagerHolder();

        expect(invokeCalls()).toHaveLength(0);

        holder.destroy();
    });

    /**
     * The connection log lives on the manager. It used to be built and destroyed by the
     * settings screen, so closing settings threw the log away and reopening showed an empty
     * pane while the proxy was still running. One manager per dashboard session outlives
     * every open and close of settings.
     */
    it("hands out the same manager for the life of the session", () => {
        const holder = new BedrockManagerHolder();

        const first = holder.get();
        const second = holder.get();

        expect(second).toBe(first);
        holder.destroy();
    });

    /**
     * The capability manager listens on window focus and schedules its own retries. It was
     * constructed inline by the settings screen and never destroyed, so every open left
     * another listener behind refreshing a manager nothing was reading any more.
     */
    it("releases the capability check on destroy", async () => {
        const holder = new BedrockManagerHolder();
        holder.get();
        await checked();

        holder.destroy();
        mockInvoke({ api_get_config: () => config() });
        // Past the focus refresh throttle, or a live listener would decline the event
        // for its own reasons and the assertion below would prove nothing.
        clock += 60_000;
        window.dispatchEvent(new Event("focus"));
        await settled();

        expect(
            invokeCalls().filter((call) => call.cmd === "api_get_config"),
            "a destroyed holder must not leave a focus listener refreshing behind it",
        ).toHaveLength(0);
    });

    // The counterpart to the test above: it is the destroy that stops the refresh, not the
    // throttle. A live holder does refresh on focus.
    it("re-checks on focus while it is alive", async () => {
        const holder = new BedrockManagerHolder();
        holder.get();
        await checked();

        mockInvoke({ api_get_config: () => config() });
        clock += 60_000;
        window.dispatchEvent(new Event("focus"));
        await settled();

        expect(invokeCalls().filter((call) => call.cmd === "api_get_config")).toHaveLength(1);
        holder.destroy();
    });

    /**
     * `get` fires `initialize`, which restores the Microsoft session before the log
     * listener's `listen` round trip has necessarily resolved. A destroy landing inside that
     * window used to leave the listener attached with nothing left to release it — writing
     * into a store nobody reads for the life of the process.
     *
     * Signing out, or otherwise leaving the dashboard, right after opening the Connect pane
     * is the way in.
     */
    it("releases a log listener that attaches after destroy", async () => {
        const holder = new BedrockManagerHolder();
        holder.get();
        holder.destroy();
        await settled();
        await settled();

        expect(
            logListenerAttached(),
            "a listener that resolved after destroy must not stay attached",
        ).toBe(false);
    });
});
