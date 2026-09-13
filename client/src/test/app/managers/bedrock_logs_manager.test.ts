import { get } from "svelte/store";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { BedrockLogEntry } from "../../../js/bindings/BedrockLogEntry";

// The manager subscribes to a webview event, and these tests are about what the buffer
// does with the entries that arrive on it. The shared mock in `src/test/tauri.ts` never
// fires, so the handler is captured here and called directly.
let emit: ((entry: BedrockLogEntry) => void) | null = null;
let unlistened = 0;

vi.mock("@tauri-apps/api/webviewWindow", () => ({
    getCurrentWebviewWindow: () => ({
        listen: async (_name: string, handler: (event: { payload: BedrockLogEntry }) => void) => {
            emit = (entry) => handler({ payload: entry });
            return () => {
                unlistened += 1;
            };
        },
        emit: async () => {},
        label: "main",
    }),
}));

const { BedrockLogsManager } = await import(
    "../../../js/app/managers/bedrock/logs/BedrockLogsManager"
);

function entry(level: string, message: string): BedrockLogEntry {
    return { timestamp_ms: 0n, level, target: "bedrock_protocol", message };
}

function send(level: string, message: string): void {
    if (!emit) throw new Error("the manager never subscribed");
    emit(entry(level, message));
}

describe("Bedrock logs manager", () => {
    beforeEach(() => {
        emit = null;
        unlistened = 0;
    });

    it("buffers every level so the debug toggle has something to reveal", async () => {
        const manager = new BedrockLogsManager();
        await manager.initialize();

        send("INFO", "proxy listening");
        send("DEBUG", "raknet frame");

        expect(get(manager.realmsLogs)).toHaveLength(2);
        manager.destroy();
    });

    /**
     * A running proxy emits `bedrock_protocol` and `raknet` at debug, which is thousands of
     * lines a minute. One flat buffer meant those flushed out every info line within seconds,
     * and with the view's debug toggle off the pane read "Nothing yet." — the log looked like
     * it kept clearing itself while the proxy ran.
     */
    it("evicts debug before info when the buffer is full", async () => {
        const manager = new BedrockLogsManager();
        await manager.initialize();

        send("INFO", "proxy listening");
        for (let i = 0; i < BedrockLogsManager.MAX_LOG_ENTRIES * 2; i += 1) {
            send("DEBUG", `raknet frame ${i}`);
        }

        const kept = get(manager.realmsLogs);
        expect(
            kept.some((line) => line.message === "proxy listening"),
            "a debug flood must not evict the info line a reader came for",
        ).toBe(true);
        expect(kept.length).toBeLessThanOrEqual(
            BedrockLogsManager.MAX_LOG_ENTRIES + BedrockLogsManager.MAX_DEBUG_ENTRIES,
        );
        expect(kept.filter((line) => BedrockLogsManager.isVerbose(line))).toHaveLength(
            BedrockLogsManager.MAX_DEBUG_ENTRIES,
        );
        manager.destroy();
    });

    it("evicts the oldest info once nothing lower is left to drop", async () => {
        const manager = new BedrockLogsManager();
        await manager.initialize();

        for (let i = 0; i < BedrockLogsManager.MAX_LOG_ENTRIES + 5; i += 1) {
            send("INFO", `line ${i}`);
        }

        const kept = get(manager.realmsLogs);
        expect(kept).toHaveLength(BedrockLogsManager.MAX_LOG_ENTRIES);
        expect(kept[0].message).toBe("line 5");
        manager.destroy();
    });

    it("keeps warnings and errors through a debug flood", async () => {
        const manager = new BedrockLogsManager();
        await manager.initialize();

        send("WARN", "backend refused once");
        send("ERROR", "backend refused again");
        for (let i = 0; i < BedrockLogsManager.MAX_LOG_ENTRIES * 2; i += 1) {
            send("TRACE", `packet ${i}`);
        }

        const levels = get(manager.realmsLogs).map((line) => line.level);
        expect(levels).toContain("WARN");
        expect(levels).toContain("ERROR");
        manager.destroy();
    });

    // Two panes mount over one manager, and the second subscribe used to return early while
    // still leaving the first listener attached. Destroy has to release what was attached.
    it("releases its listener on destroy", async () => {
        const manager = new BedrockLogsManager();
        await manager.initialize();
        manager.destroy();

        expect(unlistened).toBe(1);
    });
});
