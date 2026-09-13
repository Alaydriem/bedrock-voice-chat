import { get } from "svelte/store";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCalls, mockInvoke } from "../../tauri";

/**
 * The store, with its writes counted.
 *
 * The advertised list is cached here, and how often that cache is written is part of what
 * these tests are about — `save()` serialises the whole of `store.json`, which CLAUDE.md §20
 * documents as a round trip Android runs on the UI thread.
 */
const values = new Map<string, unknown>([["current_server", "https://bvc.example.com"]]);
let saves = 0;

vi.mock("@tauri-apps/plugin-store", () => ({
    Store: {
        load: async () => ({
            get: async (key: string) => values.get(key),
            set: async (key: string, value: unknown) => void values.set(key, value),
            delete: async (key: string) => void values.delete(key),
            save: async () => {
                saves += 1;
            },
        }),
    },
}));

const { BedrockCapabilityManager } = await import(
    "../../../js/app/managers/bedrock/BedrockCapabilityManager"
);

function configWith(servers: unknown[]) {
    return {
        config: {
            status: "Ok",
            client_id: "cid",
            protocol_version: "3.0.0",
            quic_port: 0,
            quic_ports: [1],
            bedrock: { enabled: true, servers },
        },
        client_version: "3.0.0",
        compatible: true,
        client_too_old: false,
    };
}

const OPERATOR = [
    { name: "Truly Bedrock SMP", host: "tbs7.nodecraft.gg", port: 19132, addon_mode: "net" },
];

const UNREACHABLE = {
    api_get_config: () => {
        throw new Error("unreachable");
    },
};

beforeEach(() => {
    mockInvoke({});
    values.clear();
    values.set("current_server", "https://bvc.example.com");
    saves = 0;
});

/**
 * The advertised list is fetched from `/api/config`, which is asked again on every window
 * focus — including the focus raised by closing the add-server modal. A manager rebuilt from
 * nothing, or a check that has not answered yet, left "From your server" empty until the
 * network came back.
 *
 * The last good answer is written to the store and read back on the next start, so the
 * section is populated before the first request is even sent.
 */
describe("Bedrock capability manager remembers the operator's list", () => {
    it("restores the last good list before the check answers", async () => {
        mockInvoke({ api_get_config: () => configWith(OPERATOR) });
        const first = new BedrockCapabilityManager();
        await first.refresh();
        first.destroy();

        mockInvoke(UNREACHABLE);
        const second = new BedrockCapabilityManager();
        await second.refresh();

        const entries = get(second.serverProvidedServers);
        expect(entries, "a rebuilt manager must not start with an empty list").toHaveLength(1);
        expect(entries[0].host).toBe("tbs7.nodecraft.gg");
        expect(entries[0].source).toBe("server");
        second.destroy();
    });

    it("lets a later successful check replace what was restored", async () => {
        mockInvoke({ api_get_config: () => configWith(OPERATOR) });
        const first = new BedrockCapabilityManager();
        await first.refresh();
        first.destroy();

        mockInvoke({ api_get_config: () => configWith([]) });
        const second = new BedrockCapabilityManager();
        await second.refresh();

        expect(get(second.serverProvidedServers)).toHaveLength(0);
        second.destroy();
    });

    // The cache carries the server it came from. Restoring one server's advertised worlds
    // onto another would offer worlds that server never named.
    it("does not restore one server's list onto another", async () => {
        mockInvoke({ api_get_config: () => configWith(OPERATOR) });
        const first = new BedrockCapabilityManager();
        await first.refresh();
        first.destroy();

        values.set("current_server", "https://other.example.com");
        mockInvoke(UNREACHABLE);
        const second = new BedrockCapabilityManager();
        await second.refresh();

        expect(get(second.serverProvidedServers)).toHaveLength(0);
        second.destroy();
    });

    // Written by an older build, or hand-edited. A row without `source` files itself under
    // "Yours", where the plate offers Edit and Remove that the manager refuses by id — two
    // buttons that do nothing.
    it("ignores cached rows that are not advertised entries", async () => {
        values.set("bedrock_server_provided", {
            host: "bvc.example.com",
            entries: [
                { id: "ok", name: "Kept", host: "a.example.com", port: 19132, source: "server" },
                { name: "No id", host: "b.example.com", port: 19132, source: "server" },
                { id: "no-source", name: "Unsourced", host: "c.example.com", port: 19132 },
            ],
        });
        mockInvoke(UNREACHABLE);

        const manager = new BedrockCapabilityManager();
        await manager.refresh();

        const entries = get(manager.serverProvidedServers);
        expect(entries).toHaveLength(1);
        expect(entries[0].id).toBe("ok");
        manager.destroy();
    });
});

describe("Bedrock capability manager check scheduling", () => {
    /**
     * `initialize`, the focus handler and the retry timer all call `refresh`. Concurrent
     * runs raced each other's writes, the later-resolving one winning — so a slow failure
     * could report `unknown` over an answer that had already succeeded.
     */
    it("shares one check between concurrent callers", async () => {
        mockInvoke({ api_get_config: () => configWith(OPERATOR) });
        const manager = new BedrockCapabilityManager();

        await Promise.all([manager.refresh(), manager.refresh(), manager.refresh()]);

        expect(invokeCalls().filter((call) => call.cmd === "api_get_config")).toHaveLength(1);
        expect(get(manager.status)).toBe("enabled");
        manager.destroy();
    });

    it("starts a new check once the shared one has settled", async () => {
        mockInvoke({ api_get_config: () => configWith(OPERATOR) });
        const manager = new BedrockCapabilityManager();

        await manager.refresh();
        await manager.refresh();

        expect(invokeCalls().filter((call) => call.cmd === "api_get_config")).toHaveLength(2);
        manager.destroy();
    });

    // A re-check on every window focus that rewrites an unchanged list is a whole-file
    // serialisation for nothing, at the moment a phone's UI thread is busiest.
    it("does not rewrite the cache when the list has not changed", async () => {
        mockInvoke({ api_get_config: () => configWith(OPERATOR) });
        const manager = new BedrockCapabilityManager();

        await manager.refresh();
        const afterFirst = saves;
        await manager.refresh();
        await manager.refresh();

        expect(afterFirst).toBe(1);
        expect(saves, "an unchanged advertised list must not be written again").toBe(1);
        manager.destroy();
    });

    it("writes the cache when the list does change", async () => {
        mockInvoke({ api_get_config: () => configWith(OPERATOR) });
        const manager = new BedrockCapabilityManager();
        await manager.refresh();

        mockInvoke({
            api_get_config: () =>
                configWith([
                    ...OPERATOR,
                    { name: "Second", host: "two.example.com", port: 19132, addon_mode: "net" },
                ]),
        });
        await manager.refresh();

        expect(saves).toBe(2);
        manager.destroy();
    });
});
