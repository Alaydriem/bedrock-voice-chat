import type { Store } from "@tauri-apps/plugin-store";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCalls, mockInvoke } from "../../tauri";
import "../../tauri";

const webview = vi.hoisted(() => ({
    listener: null as ((event: unknown) => void) | null,
}));

vi.mock("@tauri-apps/api/webviewWindow", () => ({
    getCurrentWebviewWindow: () => ({
        listen: async (_name: string, handler: (event: unknown) => void) => {
            webview.listener = handler;
            return () => {};
        },
        emit: async () => {},
        label: "main",
    }),
}));

const { default: ChannelManager } = await import("../../../js/app/managers/ChannelManager");
const { PlayerManager } = await import("../../../js/app/managers/PlayerManager");

beforeEach(() => {
    webview.listener = null;
});

describe("ChannelManager protect signal", () => {
    // Turning protection on reaches every client as a rename with the unchanged name. Only a
    // fresh channel list carries `protected: true`, so a rename has to re-read it.
    it("a rename re-reads the channel list", async () => {
        mockInvoke({
            api_list_channels: () => [{ id: "NR4K", name: "Night Raid", players: [], creator: "x" }],
        });
        const players = new PlayerManager("Alaydriem", "minecraft");
        const manager = new ChannelManager(players, {} as Store, "https://bvc.example.com");
        await manager.startListening();
        const before = invokeCalls().filter((c) => c.cmd === "api_list_channels").length;

        webview.listener?.({
            payload: { event_type: "rename", channel_id: "NR4K", channel_name: "Night Raid" },
        });
        await new Promise((resolve) => setTimeout(resolve, 0));

        const after = invokeCalls().filter((c) => c.cmd === "api_list_channels").length;
        expect(after).toBe(before + 1);
    });
});
