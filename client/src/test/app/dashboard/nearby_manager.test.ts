import { beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCalls, mockInvoke } from "../../tauri";

/** The feed's socket, under the test's control. */
let opened: { url: string; protocols: string[] } | null = null;
let onmessage: ((event: { data: string }) => void) | null = null;

/** Every socket the feed has opened, in order, so a reconnect is observable. */
const sockets: FakeSocket[] = [];

class FakeSocket {
    onmessage: ((event: { data: string }) => void) | null = null;
    onclose: (() => void) | null = null;
    onerror: (() => void) | null = null;

    constructor(url: string, protocols: string[]) {
        opened = { url, protocols };
        sockets.push(this);
        // Registered on the instance the manager holds, so `deliver` reaches the same one.
        setTimeout(() => (onmessage = this.onmessage), 0);
    }

    close(): void {}
}

vi.stubGlobal("WebSocket", FakeSocket);

vi.mock("@tauri-apps/api/webviewWindow", () => ({
    getCurrentWebviewWindow: () => ({
        listen: async () => () => {},
    }),
}));

const { NearbyManager } = await import("../../../js/app/dashboard/NearbyManager");

function store() {
    const values: Record<string, unknown> = {};
    return {
        get: async (key: string) => values[key],
        set: async (key: string, value: unknown) => {
            values[key] = value;
        },
        save: async () => {},
        values,
    } as never as import("@tauri-apps/plugin-store").Store & { values: Record<string, unknown> };
}

function entry(name: string, distance: number, presence = "voice") {
    return { name, presence, bearing_deg: 90, distance, elevation: 0 };
}

async function deliver(seq: number, positions: unknown[]): Promise<void> {
    await vi.waitFor(() => expect(onmessage).not.toBeNull());
    onmessage!({ data: JSON.stringify({ seq, positions }) });
}

describe("NearbyManager", () => {
    beforeEach(() => {
        opened = null;
        onmessage = null;
        sockets.length = 0;
        mockInvoke({
            api_websocket_ticket: () => ({ ticket: "abc", expires_in: 60 }),
        });
    });

    it("offers the ticket as a subprotocol, because a browser cannot set a header", async () => {
        const nearby = new NearbyManager(store());
        await nearby.start("https://voice.example.com", 48);

        expect(opened?.url).toBe("wss://voice.example.com/api/websocket/positions");
        // And not in the URL, where it would land in every access log on the way.
        expect(opened?.protocols).toEqual(["ticket.abc", "bvc.positions.v1"]);
        nearby.stop();
    });

    it("splits the roster from the ring at the server's own voice range", async () => {
        const nearby = new NearbyManager(store());
        let inEarshot: readonly { name: string }[] = [];
        let approaching: readonly { name: string }[] = [];
        nearby.inEarshot.subscribe((v) => (inEarshot = v));
        nearby.approaching.subscribe((v) => (approaching = v));

        await nearby.start("https://voice.example.com", 48);
        await deliver(1, [entry("minecraft:Close", 40), entry("minecraft:Far", 60)]);

        // 48 is the server's `broadcast_range`, so the line the roster draws is the line a
        // voice actually stops at rather than an approximation of it.
        expect(inEarshot.map((p) => p.name)).toEqual(["minecraft:Close"]);
        expect(approaching.map((p) => p.name)).toEqual(["minecraft:Far"]);
        nearby.stop();
    });

    it("orders everyone nearest first, which is what the card split relies on", async () => {
        const nearby = new NearbyManager(store());
        let all: readonly { distance: number }[] = [];
        nearby.players.subscribe((v) => (all = v));

        await nearby.start("https://voice.example.com", 48);
        await deliver(1, [
            entry("minecraft:C", 30),
            entry("minecraft:A", 5),
            entry("minecraft:B", 12),
        ]);

        expect(all.map((p) => p.distance)).toEqual([5, 12, 30]);
        nearby.stop();
    });

    it("keeps a silent player, because the feed reports them whether they talk or not", async () => {
        const nearby = new NearbyManager(store());
        let all: readonly unknown[] = [];
        nearby.players.subscribe((v) => (all = v));

        await nearby.start("https://voice.example.com", 48);
        await deliver(1, [entry("minecraft:Quiet", 10)]);
        await deliver(2, [entry("minecraft:Quiet", 10)]);

        // The reason membership comes from the feed at all: coordinates ride on a player's own
        // audio frames, so somebody standing next to you in silence has no position anywhere
        // on this machine.
        expect(all).toHaveLength(1);
        nearby.stop();
    });

    it("discards a snapshot that arrives after a newer one", async () => {
        const nearby = new NearbyManager(store());
        let all: readonly { name: string }[] = [];
        nearby.players.subscribe((v) => (all = v));

        await nearby.start("https://voice.example.com", 48);
        await deliver(5, [entry("minecraft:Now", 10)]);
        await deliver(2, [entry("minecraft:Stale", 10), entry("minecraft:Now", 10)]);

        // A reordered frame would move every card backwards for a tick.
        expect(all.map((p) => p.name)).toEqual(["minecraft:Now"]);
        nearby.stop();
    });

    /**
     * `seq` counts per socket, starting at 1, so the ordering guard has to forget it with the
     * socket that issued it.
     *
     * Carrying it across a reconnect made the guard reject every frame of the new socket until
     * its counter passed the old one's high-water mark, so a replacement socket discarded
     * everything up to that mark — including the arrival that prompted it.
     */
    it("forgets the sequence when the socket is replaced", async () => {
        vi.useFakeTimers();
        try {
            const nearby = new NearbyManager(store());
            let all: readonly { name: string }[] = [];
            nearby.players.subscribe((v) => (all = v));

            const starting = nearby.start("https://voice.example.com", 48);
            await vi.advanceTimersByTimeAsync(0);
            await starting;

            const first = sockets.at(-1);
            expect(first?.onmessage).toBeTruthy();
            first!.onmessage!({
                data: JSON.stringify({ seq: 60, positions: [entry("minecraft:A", 10)] }),
            });
            expect(all).toHaveLength(1);

            // Not an error: the server closes a feed it has nothing to say on, and reconnecting
            // is how it comes back when it does.
            first!.onclose!();
            await vi.advanceTimersByTimeAsync(3_100);
            expect(sockets).toHaveLength(2);

            sockets.at(-1)!.onmessage!({
                data: JSON.stringify({ seq: 1, positions: [entry("minecraft:B", 10)] }),
            });

            expect(all.map((p) => p.name)).toContain("minecraft:B");
            nearby.stop();
        } finally {
            vi.useRealTimers();
        }
    });

    it("records a player on first sight so their volume outlives them", async () => {
        const s = store();
        const nearby = new NearbyManager(s);

        await nearby.start("https://voice.example.com", 48);
        await deliver(1, [entry("minecraft:Petra", 10)]);
        await vi.waitFor(() => expect(s.values["player_gain_store"]).toBeDefined());

        const gains = s.values["player_gain_store"] as Record<string, { last_seen?: number }>;
        // Keyed on the canonical identity, which is what the mixer's gain projection and the
        // persisted store resolve against. A bare key here would write an entry that nothing
        // downstream ever looks up.
        expect(gains["minecraft:Petra"]).toBeDefined();
        expect(typeof gains["minecraft:Petra"].last_seen).toBe("number");
        expect(gains["Petra"]).toBeUndefined();
        nearby.stop();
    });

    /**
     * Tickets are single-use and issuing one revokes the identity's previous, so two attempts
     * overlapping is not merely wasteful: the second revokes the first's credential, the first
     * is refused at redeem time, and its retry revokes the second's in turn. The roster stayed
     * empty for as long as that ran.
     */
    it("spends one ticket per socket", async () => {
        const nearby = new NearbyManager(store());
        await nearby.start("https://voice.example.com", 48);

        expect(invokeCalls().filter((c) => c.cmd === "api_websocket_ticket")).toHaveLength(1);
        expect(sockets).toHaveLength(1);
        nearby.stop();
    });

    it("abandons an attempt that stop() overtook while it waited for a ticket", async () => {
        let release: (ticket: unknown) => void = () => {};
        mockInvoke({
            api_websocket_ticket: () =>
                new Promise((resolve) => {
                    release = resolve;
                }),
        });

        const nearby = new NearbyManager(store());
        const starting = nearby.start("https://voice.example.com", 48);
        nearby.stop();
        release({ ticket: "abc", expires_in: 60 });
        await starting;

        // A socket opened after the feed was stopped belongs to nobody, and the ticket it
        // spends is the one the replacement feed needs.
        expect(sockets).toHaveLength(0);
    });

    it("reports somebody in the world but not on voice", async () => {
        const nearby = new NearbyManager(store());
        let all: readonly { presence: string }[] = [];
        nearby.players.subscribe((v) => (all = v));

        await nearby.start("https://voice.example.com", 48);
        await deliver(1, [entry("minecraft:NoBvc", 10, "game")]);

        expect(all[0].presence).toBe("game");
        nearby.stop();
    });
});
