import { get } from "svelte/store";
import { beforeEach, describe, expect, it } from "vitest";
import { mockInvoke } from "../../tauri";
import { BedrockProxyManager } from "../../../js/app/managers/bedrock/proxy/BedrockProxyManager";
import type { ProxyServerEntry } from "../../../js/app/managers/bedrock/ProxyServerEntry";

function manager(): BedrockProxyManager {
    return new BedrockProxyManager({ setStatus: () => {} });
}

function offered(host: string, name = host): ProxyServerEntry {
    return { id: `server:${host}:19132`, name, host, port: 19132, source: "server" };
}

beforeEach(() => {
    mockInvoke({});
});

describe("Bedrock proxy manager server list", () => {
    /**
     * Adding your own entry for a world the operator also advertises used to delete the
     * operator's row: the merge dropped any advertised entry whose host and port matched one
     * of yours. Where the operator advertises a single world — the common case — the whole
     * "From your server" section disappeared the moment you saved a server of your own.
     *
     * Both are kept. They live in separate sections that say where each came from, so a
     * reader can tell them apart, and the operator's word about its own world survives.
     */
    it("keeps the operator's entry when you add your own for the same address", async () => {
        const proxy = manager();
        proxy.setServerProvidedServers([offered("play.example.com", "Operator world")]);
        await proxy.addProxyServer("My copy", "play.example.com", 19132);

        const sorted = get(proxy.sortedProxyServers);

        expect(
            sorted.filter((entry) => entry.source === "server"),
            "a saved entry of your own must not delete the operator's",
        ).toHaveLength(1);
        expect(sorted.filter((entry) => entry.source !== "server")).toHaveLength(1);
    });

    it("keeps the rest of the operator's list when one address collides", async () => {
        const proxy = manager();
        proxy.setServerProvidedServers([
            offered("one.example.com"),
            offered("two.example.com"),
            offered("three.example.com"),
        ]);
        await proxy.addProxyServer("My copy", "two.example.com", 19132);

        expect(get(proxy.sortedProxyServers).filter((e) => e.source === "server")).toHaveLength(3);
    });

    // Same host, different port, is a different world. Nothing about it is a collision.
    it("treats a different port as a different entry", async () => {
        const proxy = manager();
        proxy.setServerProvidedServers([offered("play.example.com")]);
        await proxy.addProxyServer("Other port", "play.example.com", 19133);

        expect(get(proxy.sortedProxyServers)).toHaveLength(2);
    });

    /**
     * With both entries present, a running session's address matches two rows and nothing
     * records which one it was started from. Yours wins the tie-break.
     */
    it("puts the running chip on your entry rather than the operator's copy", async () => {
        const proxy = manager();
        proxy.setServerProvidedServers([offered("play.example.com")]);
        const mine = await proxy.addProxyServer("My copy", "play.example.com", 19132);

        proxy.applyStatus({
            host: "play.example.com",
            port: 19132,
            listenPort: 28282,
            running: true,
        });

        expect(get(proxy.activeProxyId)).toBe(mine.id);
    });

    it("falls back to the operator's entry when you saved none", () => {
        const proxy = manager();
        proxy.setServerProvidedServers([offered("play.example.com")]);

        proxy.applyStatus({
            host: "play.example.com",
            port: 19132,
            listenPort: 28282,
            running: true,
        });

        expect(get(proxy.activeProxyId)).toBe("server:play.example.com:19132");
    });

    /**
     * `updateProxyServer` copied name, host, port and protocolVersion out of the patch but
     * not addonMode, so the "Proxy all events" checkbox could never be changed on a server
     * that already existed: the edit appeared to save and the entry silently kept the mode
     * it was created with.
     *
     * The consequence was invisible and total. A world left on `net` resolves to
     * `RelayOnlyDispatch`, which has no PlaySound arm at all, so the jukebox bus and the
     * `bvc:ctl:` control plane were both dropped without a log line.
     */
    it("saves a change to the addon mode on an existing entry", async () => {
        const proxy = manager();
        const mine = await proxy.addProxyServer("BDS Local", "10.0.0.1", 19135, undefined, "net");

        await proxy.updateProxyServer(mine.id, { addonMode: "no_net" });

        const saved = get(proxy.sortedProxyServers).find((entry) => entry.id === mine.id);
        expect(saved?.addonMode, "ticking Proxy all events must persist").toBe("no_net");
    });

    it("leaves the addon mode alone when the patch does not mention it", async () => {
        const proxy = manager();
        const mine = await proxy.addProxyServer("Aternos", "a.aternos.me", 19132, undefined, "no_net");

        await proxy.updateProxyServer(mine.id, { name: "Renamed" });

        const saved = get(proxy.sortedProxyServers).find((entry) => entry.id === mine.id);
        expect(saved?.name).toBe("Renamed");
        expect(saved?.addonMode, "an unrelated edit must not reset the mode").toBe("no_net");
    });
});
