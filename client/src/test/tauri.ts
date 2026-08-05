import { vi } from "vitest";

interface InvokeCall {
    readonly cmd: string;
    readonly args: unknown;
}

const calls: InvokeCall[] = [];
let handlers: Record<string, (args: never) => unknown> = {};

/**
 * The IPC boundary, under the test's control.
 *
 * Mocking here is the feature, not the compromise: "the probe timed out and Sign in
 * is still clickable" has to be asserted deterministically, and a real network will
 * not reliably time out on cue.
 *
 * An unregistered command rejects rather than returning undefined, so a screen that
 * starts calling something new fails loudly instead of silently reading a missing
 * value.
 */
vi.mock("@tauri-apps/api/core", () => ({
    invoke: vi.fn(async (cmd: string, args: unknown) => {
        calls.push({ cmd, args });
        const handler = handlers[cmd];
        if (!handler) throw new Error(`unmocked invoke: ${cmd}`);
        return handler(args as never);
    }),
}));

/**
 * The key/value store, held in memory.
 *
 * The plugin reaches straight for the real IPC rather than the mocked one, so it has to
 * be replaced whole. A test file that wants particular values still mocks it itself.
 */
vi.mock("@tauri-apps/plugin-store", () => {
    const values = new Map<string, unknown>([["current_server", "https://bvc.example.com"]]);
    return {
        Store: {
            load: async () => ({
                get: async (key: string) => values.get(key),
                set: async (key: string, value: unknown) => void values.set(key, value),
                delete: async (key: string) => void values.delete(key),
                save: async () => {},
            }),
        },
    };
});

/**
 * The webview, which does not exist here.
 *
 * `getCurrentWebviewWindow` reads `window.__TAURI_INTERNALS__.metadata` and throws
 * without it, so any manager that subscribes to an app event takes its whole screen
 * down under test.
 */
vi.mock("@tauri-apps/api/webviewWindow", () => ({
    getCurrentWebviewWindow: () => ({
        listen: async () => () => {},
        emit: async () => {},
        label: "main",
    }),
}));

vi.mock("@tauri-apps/plugin-log", () => ({
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
}));

export function mockInvoke(next: Record<string, (args: never) => unknown>): void {
    handlers = next;
    calls.length = 0;
}

export function invokeCalls(): readonly InvokeCall[] {
    return calls;
}
