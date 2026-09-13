import { render, waitFor } from "@testing-library/svelte";
import { readable, writable, type Writable } from "svelte/store";
import { beforeEach, describe, expect, it } from "vitest";
import { mockInvoke } from "../tauri";
import type { BedrockManager } from "../../js/app/managers/bedrock/BedrockManager";

const { default: ConnectPane } = await import("../../components/settings/panes/ConnectPane.svelte");

/**
 * A stand-in for the Bedrock manager whose capability check can be held mid-flight, which is
 * what these tests are about.
 */
function stub(checking: boolean, servers: unknown[]) {
    const list: Writable<unknown[]> = writable(servers);
    return {
        isAuthenticated: readable(true),
        isRestoringAuth: readable(false),
        capability: {
            status: readable("enabled"),
            isChecking: readable(checking),
            refresh: async () => {},
        },
        sortedProxyServers: list,
        proxyFavorites: readable(new Set<string>()),
        activeProxyId: readable<string | null>(null),
        proxyRunning: readable(false),
        sortedRealms: readable([]),
        isLoadingRealms: readable(false),
        favorites: readable(new Set<string>()),
        activeRealmId: readable<bigint | null>(null),
        activeRealmName: readable(""),
        showLoginModal: readable(false),
        deviceCode: readable(""),
        deviceUrl: readable(""),
        loginError: readable(""),
        codeCopied: readable(false),
        realmsLogs: readable([]),
        logsExpanded: readable(false),
        closeLoginModal: async () => {},
        copyDeviceCode: async () => {},
        openLoginUrl: async () => {},
        clearLogs: () => {},
        toggleLogs: () => {},
        initializeRealmsAccess: async () => {},
        listProtocolVersions: async () => [],
        openLoginModal: async () => {},
        stopProxy: async () => {},
        stopRealms: async () => {},
        connectToProxyServer: async () => {},
        connectToRealm: async () => {},
        toggleProxyFavorite: async () => {},
        toggleFavorite: async () => {},
        addProxyServer: async () => ({ id: "x", name: "n", host: "h", port: 19132 }),
        updateProxyServer: async () => {},
        deleteProxyServer: async () => {},
    } as unknown as BedrockManager;
}

function mount(bedrock: BedrockManager) {
    const host = document.createElement("div");
    document.body.append(host);
    render(ConnectPane as never, { target: host, props: { bedrock, mobile: false } } as never);
    return {
        host,
        text: () => host.textContent ?? "",
        heads: () =>
            [...host.querySelectorAll(".rad-section__head")].map((h) => h.textContent?.trim()),
    };
}

beforeEach(() => {
    mockInvoke({
        bedrock_get_status: () => ({
            proxy_running: false,
            realms_running: false,
            xbox_authenticated: true,
            proxy_target_host: null,
            proxy_target_port: null,
            proxy_listen_port: 28282,
            active_realm_id: null,
            active_realm_name: null,
            proxy_started_at: null,
        }),
    });
});

/**
 * Saving a server closes a modal, which raises a window focus, which re-runs the capability
 * check. While that check was in flight the whole list was replaced by a loader — so adding a
 * server made the operator's section disappear for as long as the server took to answer, and
 * for good if it never did.
 */
describe("Connect pane during a capability re-check", () => {
    it("keeps the rows it already has while a re-check is in flight", async () => {
        const view = mount(
            stub(true, [
                {
                    id: "server:play.example.com:19132",
                    name: "Operator world",
                    host: "play.example.com",
                    port: 19132,
                    source: "server",
                },
                { id: "mine", name: "My copy", host: "play.example.com", port: 19132 },
            ]),
        );

        await waitFor(() => expect(view.text()).toContain("Operator world"));
        expect(view.heads()).toEqual(["From your server", "Yours"]);
        expect(view.text()).toContain("My copy");
    });

    // With nothing to show yet, the loader is still the right answer — it says the answer is
    // coming rather than that there is none.
    it("shows the loader on a first check with no rows yet", async () => {
        const view = mount(stub(true, []));

        await waitFor(() => expect(view.host.querySelector(".rad-skeleton")).not.toBeNull());
        expect(view.text()).not.toContain("No servers yet");
    });
});
