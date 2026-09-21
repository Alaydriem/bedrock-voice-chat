import type { Store } from "@tauri-apps/plugin-store";
import { get } from "svelte/store";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCalls, mockInvoke } from "../../tauri";
import "../../tauri";

// The webview's own mock returns a no-op listener, so the handler can never be
// driven. This one keeps it, which is the only way to deliver a channel_event.
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

const { default: ChannelManager } = await import(
  "../../../js/app/managers/ChannelManager"
);
const { PlayerManager } = await import("../../../js/app/managers/PlayerManager");

const ME = "minecraft:Alaydriem";
const FIRST = "AB12-CD34";
const SECOND = "EF56-GH78";

function channel(id: string, name: string, players: string[]) {
  return { id, name, players, creator: ME };
}

async function manager(channels: ReturnType<typeof channel>[]) {
  mockInvoke({
    api_list_channels: () => channels,
    api_channel_event: () => true,
  });
  const players = new PlayerManager("Alaydriem", "minecraft");
  const channelManager = new ChannelManager(players, {} as Store, "https://bvc.example.com");
  await channelManager.startListening();
  return channelManager;
}

function deliver(payload: Record<string, unknown>): void {
  if (!webview.listener) {
    throw new Error("no channel_event listener was registered");
  }
  webview.listener({ payload });
}

beforeEach(() => {
  webview.listener = null;
});

describe("ChannelManager membership tracking", () => {
  // The addon's panel can move this player between groups. That arrives as a
  // channel_event, not as a call into this manager, and the manager has to adopt it
  // — otherwise the next join from the webview has nothing to leave.
  it("adopts a group it is told this client joined", async () => {
    const channelManager = await manager([channel(FIRST, "Sculk Striders", [])]);

    deliver({
      event_type: "join",
      channel_id: FIRST,
      channel_name: "Sculk Striders",
      creator: ME,
      player_name: ME,
    });

    expect(get(channelManager.currentUserChannelId)).toBe(FIRST);
  });

  // The symptom this guards: the player joins from the in-game panel, then toggles a
  // different group in the app and is left sitting in both. Joining is a MOVE, and a
  // move that does not know where it started leaves the old membership behind.
  it("leaves the group the addon put it in before joining another", async () => {
    const channelManager = await manager([
      channel(FIRST, "Sculk Striders", []),
      channel(SECOND, "Ember Ravagers", []),
    ]);

    deliver({
      event_type: "join",
      channel_id: FIRST,
      channel_name: "Sculk Striders",
      creator: ME,
      player_name: ME,
    });

    await channelManager.joinChannel(SECOND, "Alaydriem");

    const left = invokeCalls().filter(
      (call) =>
        call.cmd === "api_channel_event" &&
        (call.args as { channelId: string; event: { event: string } }).event
          .event === "Leave",
    );
    expect(
      left.map((call) => (call.args as { channelId: string }).channelId),
    ).toEqual([FIRST]);
    expect(get(channelManager.currentUserChannelId)).toBe(SECOND);
  });

  // Another player's join is not this client's. Adopting it would claim membership of
  // a group this client was never put in.
  it("does not adopt a group another player joined", async () => {
    const channelManager = await manager([channel(FIRST, "Sculk Striders", [])]);

    deliver({
      event_type: "join",
      channel_id: FIRST,
      channel_name: "Sculk Striders",
      creator: ME,
      player_name: "minecraft:Petra",
    });

    expect(get(channelManager.currentUserChannelId)).toBeNull();
  });
});
