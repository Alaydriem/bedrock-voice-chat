import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { invokeCalls, mockInvoke } from "../../tauri";

vi.mock("tauri-plugin-audio-permissions", () => ({
    checkPermission: async () => ({ granted: true }),
    startForegroundService: async () => ({ started: true }),
    stopForegroundService: async () => ({}),
    updateNotification: async () => ({}),
    isServiceRunning: async () => ({ running: true }),
    PermissionType: { Audio: "audio", Notification: "notification" },
}));

// A device that already finished setup. A fresh store reads as unfinished, and the boot would
// land on /setup before it reached the voice path.
vi.mock("../../../js/app/setup/SetupFlow", () => ({
    default: class {
        async initialize(): Promise<void> {}
        isComplete(): boolean {
            return true;
        }
    },
}));

vi.mock("../../../js/app/services/PublicServerConfig", () => ({
    PublicServerConfig: { read: async () => ({}) },
}));

import Dashboard from "../../../js/app/dashboard";

const LEFT = "https://left.example";
const JOINED = "https://joined.example";

const credentials = {
    gamertag: "Alaydriem",
    certificate: "",
    certificate_key: "",
    certificate_ca: "",
    quic_connect_string: "8443",
};

/**
 * The backend as the rail's switch finds it: both audio streams still running, and the voice
 * link still up to the server being left. A webview reload does not touch either.
 */
function backendStillOn(connected: string | null): void {
    const answers: Record<string, (args: never) => unknown> = {
        is_stopped: () => false,
        connected_voice_server: () => connected,
        get_credentials: () => ({ ...credentials }),
        is_certificate_expired: () => false,
        is_recording: () => false,
        api_get_config: () => ({ config: {} }),
        get_audio_device: () => ({ name: "default" }),
        websocket_internal_endpoint: () => {
            throw new Error("push listener not bound");
        },
    };

    mockInvoke(
        new Proxy(answers, {
            get: (target, cmd: string) => target[cmd] ?? (() => undefined),
        }),
    );
}

function dialed(): string[] {
    return invokeCalls()
        .filter((call) => call.cmd === "change_network_stream")
        .map((call) => (call.args as { server: string }).server);
}

describe("Dashboard voice path on a server switch", () => {
    beforeEach(() => {
        window.history.replaceState({}, "", `/dashboard?server=${encodeURIComponent(JOINED)}`);
    });

    afterEach(() => {
        window.history.replaceState({}, "", "/");
    });

    it("dials the server switched to while the streams are still running", async () => {
        backendStillOn(LEFT);

        await new Dashboard().initialize();

        expect(dialed()).toEqual([JOINED]);
    });

    it("dials when the streams are running but no voice link is up", async () => {
        backendStillOn(null);

        await new Dashboard().initialize();

        expect(dialed()).toEqual([JOINED]);
    });

    it("leaves a link to the same server alone", async () => {
        backendStillOn(JOINED);

        await new Dashboard().initialize();

        expect(dialed()).toEqual([]);
    });
});
