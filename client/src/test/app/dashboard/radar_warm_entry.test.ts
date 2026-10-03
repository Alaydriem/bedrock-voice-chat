import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { mockInvoke } from "../../tauri";

vi.mock("tauri-plugin-audio-permissions", () => ({
    checkPermission: async () => ({ granted: true }),
    startForegroundService: async () => ({ started: true }),
    stopForegroundService: async () => ({}),
    updateNotification: async () => ({}),
    isServiceRunning: async () => ({ running: true }),
    PermissionType: { Audio: "audio", Notification: "notification" },
}));

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

const SERVER = "https://voice.example";

/**
 * A warm re-entry: both streams running and the voice link already up to this server, so the
 * boot skips the stream setup that also reads `/api/config`.
 */
function warmEntry(radar: boolean): void {
    const answers: Record<string, (args: never) => unknown> = {
        is_stopped: () => false,
        connected_voice_server: () => SERVER,
        get_credentials: () => ({
            gamertag: "Alaydriem",
            certificate: "",
            certificate_key: "",
            certificate_ca: "",
            quic_connect_string: "8443",
        }),
        is_certificate_expired: () => false,
        is_recording: () => false,
        api_get_config: () => ({ config: { radar: { enabled: radar } } }),
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

describe("Dashboard radar on a warm re-entry", () => {
    beforeEach(() => {
        window.history.replaceState({}, "", `/dashboard?server=${encodeURIComponent(SERVER)}`);
    });

    afterEach(() => {
        window.history.replaceState({}, "", "/");
    });

    it("keeps radar off for a server that turned it off", async () => {
        warmEntry(false);

        const dashboard = new Dashboard();
        await dashboard.initialize();

        expect(dashboard.radar).toBe(false);
    });

    it("keeps radar on for a server that leaves it on", async () => {
        warmEntry(true);

        const dashboard = new Dashboard();
        await dashboard.initialize();

        expect(dashboard.radar).toBe(true);
    });
});
