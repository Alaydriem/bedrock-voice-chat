import { beforeEach, describe, expect, it, vi } from "vitest";
import { mockInvoke } from "../../tauri";

let stored: Record<string, unknown> = {};

vi.mock("../../../js/app/services/AppStore", () => ({
    AppStore: {
        load: async () => ({
            get: async (key: string) => stored[key],
            set: async (key: string, value: unknown) => {
                stored[key] = value;
            },
            save: async () => {},
        }),
    },
}));

let mobile = false;
vi.mock("../../../js/app/utils/PlatformDetector", () => ({
    default: class {
        async checkMobile(): Promise<boolean> {
            return mobile;
        }
    },
}));

const { WebSocketSettingsManager } = await import(
    "../../../js/app/managers/settings/WebSocketSettingsManager"
);

function read<T>(store: { subscribe: (run: (v: T) => void) => () => void }): T {
    let value!: T;
    store.subscribe((v) => (value = v))();
    return value;
}

beforeEach(() => {
    stored = {};
    mobile = false;
    mockInvoke({
        update_websocket_config: () => null,
        restart_websocket_external: () => null,
        generate_encryption_key: () => "generated-key",
    });
});

describe("WebSocketSettingsManager migration", () => {
    // A user who never turned the server on never expressed an external posture.
    it("refuses external access for a config that was disabled", async () => {
        stored.websocket_server = {
            enabled: false,
            localhost_only: false,
            port: 9595,
            key: "k",
        };
        const manager = new WebSocketSettingsManager();
        await manager.initialize();
        expect(read(manager.allowExternal)).toBe(false);
    });

    it("carries over a deliberate external posture", async () => {
        stored.websocket_server = {
            enabled: true,
            localhost_only: false,
            port: 9595,
            key: "k",
        };
        const manager = new WebSocketSettingsManager();
        await manager.initialize();
        expect(read(manager.allowExternal)).toBe(true);
    });

    // On mobile `localhost_only` was forced false rather than chosen, so migrating it naively
    // would switch external access on for every existing phone install and produce a
    // local-network permission prompt nobody asked for.
    it("refuses external access on mobile whatever was stored", async () => {
        mobile = true;
        stored.websocket_server = {
            enabled: true,
            localhost_only: false,
            port: 9595,
            key: "k",
        };
        const manager = new WebSocketSettingsManager();
        await manager.initialize();
        expect(read(manager.allowExternal)).toBe(false);
    });

    // With no enable step there is no longer a moment that mints a key, so first boot has to.
    it("generates a key when none is stored", async () => {
        const manager = new WebSocketSettingsManager();
        await manager.initialize();
        expect(read(manager.authKey)).toBe("generated-key");
        expect((stored.websocket_server as { key: string }).key).toBe("generated-key");
    });

    it("keeps a key that is already stored", async () => {
        stored.websocket_server = {
            enabled: true,
            localhost_only: true,
            allow_external: false,
            port: 9595,
            key: "existing",
        };
        const manager = new WebSocketSettingsManager();
        await manager.initialize();
        expect(read(manager.authKey)).toBe("existing");
    });
});
