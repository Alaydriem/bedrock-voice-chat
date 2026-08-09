import { render, waitFor } from "@testing-library/svelte";
import { readable, writable, type Writable } from "svelte/store";
import { beforeEach, describe, expect, it } from "vitest";
import { mockInvoke } from "../tauri";
import type { BedrockManager } from "../../js/app/managers/bedrock/BedrockManager";
import type { BedrockCapabilityStatus } from "../../js/app/managers/bedrock/BedrockCapabilityManager";

const { default: ProxyPane } = await import("../../components/settings/panes/ProxyPane.svelte");
const { default: RealmsPane } = await import("../../components/settings/panes/RealmsPane.svelte");

interface Knobs {
    authed?: boolean;
    capability?: BedrockCapabilityStatus | null;
    restoring?: boolean;
    transferPort?: number | null;
    dnsOverrideHost?: string | null;
}

/**
 * A stand-in for the Bedrock manager.
 *
 * The real one opens an Xbox session on construction. What these tests are about is
 * which surface a pane shows for a given answer, so the answers are supplied directly.
 */
function stub({
    authed = true,
    capability = "enabled",
    restoring = false,
    transferPort = null,
    dnsOverrideHost = null,
}: Knobs = {}) {
    const realms: Writable<unknown[]> = writable([
        { id: 1n, name: "Alaydriem's Realm", motd: "Survival", state: "OPEN", owner_uuid: "u" },
        { id: 2n, name: "Hearthhold", motd: "Building", state: "CLOSED", owner_uuid: "u" },
    ]);
    const servers: Writable<unknown[]> = writable([
        { id: "b1", name: "Alaydriem's SMP", host: "mc.alaydriem.com", port: 19132 },
        { id: "b2", name: "Hearthhold", host: "play.hearthhold.net", port: 19132, source: "server" },
    ]);

    return {
        isAuthenticated: readable(authed),
        capability: {
            status: readable(capability),
            isChecking: readable(false),
            serverHost: readable("bvc.example.com"),
            transferPort: readable(transferPort),
            dnsOverrideHost: readable(dnsOverrideHost),
            refresh: async () => {},
        },
        sortedProxyServers: servers,
        proxyFavorites: readable(new Set<string>()),
        activeProxyId: readable<string | null>(null),
        proxyRunning: readable(false),
        interfaces: readable([{ name: "Ethernet", ip: "192.168.1.24", is_ipv4: true }]),
        listenPort: readable(19132),
        sortedRealms: realms,
        isLoadingRealms: readable(false),
        favorites: readable(new Set<string>()),
        activeRealmId: readable<bigint | null>(null),
        activeRealmName: readable(""),
        // The sign-in modal reads these; without them subscribing throws and takes the pane
        // down with it.
        showLoginModal: readable(false),
        deviceCode: readable("JQ4H-7TDM"),
        deviceUrl: readable("microsoft.com/link"),
        loginError: readable(""),
        codeCopied: readable(false),
        isRestoringAuth: readable(restoring),
        closeLoginModal: async () => {},
        copyDeviceCode: async () => {},
        openLoginUrl: async () => {},
        // The log panel reads these.
        realmsLogs: readable([
            { timestamp_ms: 1_753_732_440_000n, level: "INFO", target: "proxy", message: "listening" },
        ]),
        logsExpanded: readable(false),
        clearLogs: () => {},
        toggleLogs: () => {},
        addProxyServer: async () => ({ id: "b3", name: "n", host: "h", port: 19132 }),
        updateProxyServer: async () => {},
        initialize: async () => {},
        listProtocolVersions: async () => [{ protocol: 800, label: '1.21.100' }],
        loadInterfaces: async () => {},
        initializeRealmsAccess: async () => {},
        openLoginModal: async () => {},
        stopProxy: async () => {},
        stopRealms: async () => {},
        connectToProxyServer: async () => {},
        connectToRealm: async () => {},
        toggleProxyFavorite: async () => {},
        toggleFavorite: async () => {},
        deleteProxyServer: async () => {},
    } as unknown as BedrockManager;
}

function mount(component: unknown, bedrock: BedrockManager, mobile = false) {
    const host = document.createElement("div");
    document.body.append(host);
    render(component as never, { target: host, props: { bedrock, mobile } } as never);
    return { host, text: () => host.textContent ?? "" };
}

beforeEach(() => {
    mockInvoke({
        bedrock_get_status: () => ({
            proxy_running: false,
            realms_running: false,
            xbox_authenticated: true,
            proxy_target_host: null,
            proxy_target_port: null,
            proxy_listen_port: 19132,
            active_realm_id: null,
            active_realm_name: null,
            proxy_started_at: null,
        }),
    });
});

describe("Bedrock panes", () => {
    // One omission explained three reports: signed-in state forgotten, favourites forgotten,
    // and an empty server list. `initialize` is what loads both managers from the store and
    // restores the Microsoft session, and the panes never called it.
    it("shows the log panel, which only exists once the manager is loaded", async () => {
        const view = mount(ProxyPane, stub());
        await waitFor(() => expect(view.text()).toContain("Connection log"));
    });

    // A log is the longest thing on the pane. Open by default on a phone it puts every
    // control above it behind a scroll.
    it("collapses the log by default on mobile and opens it on desktop", async () => {
        const phone = mount(ProxyPane, stub(), true);
        await waitFor(() => expect(phone.host.querySelector(".rad-disclosure")).not.toBeNull());
        expect(phone.host.querySelector(".rad-disclosure")?.classList.contains("is-open")).toBe(false);

        const desk = mount(ProxyPane, stub());
        await waitFor(() => expect(desk.host.querySelector(".rad-disclosure")).not.toBeNull());
        expect(desk.host.querySelector(".rad-disclosure")?.classList.contains("is-open")).toBe(true);
    });

    // Narration the pane already demonstrates. Both had a paragraph restating the callout
    // below it.
    it("carries no section narration", async () => {
        const view = mount(ProxyPane, stub());
        await waitFor(() => expect(view.text()).toContain("Where you play"));
        expect(view.text()).not.toContain("Bedrock cannot tell BVC where you are standing");

        const realms = mount(RealmsPane, stub());
        await waitFor(() => expect(realms.text()).toContain("Your Realms"));
        expect(realms.text()).not.toContain("Read from the Microsoft account");
    });
});

// A stored session is read back on start, so `authed` is false for a moment before it is
// true. Both panes took that moment as an answer and showed the sign-in card, which then
// swapped itself out.
describe("Bedrock panes while the session is being read back", () => {
    it("shows a loader rather than a sign-in card", () => {
        for (const pane of [ProxyPane, RealmsPane]) {
            const view = mount(pane, stub({ authed: false, restoring: true }));
            expect(view.text()).not.toContain("Sign in with Microsoft");
            expect(view.host.querySelector(".rad-loader")).not.toBeNull();
        }
    });

    it("shows the sign-in card once the read back finds nothing", () => {
        for (const pane of [ProxyPane, RealmsPane]) {
            const view = mount(pane, stub({ authed: false, restoring: false }));
            expect(view.text()).toContain("Sign in with Microsoft");
        }
    });
});

describe("ProxyPane", () => {
    // The proxy joins the backend as you. Everything below the gate is meaningless
    // without an account, and showing it invites somebody to configure a dead end.
    it("shows nothing but the sign-in until Microsoft is connected", () => {
        const view = mount(ProxyPane, stub({ authed: false }));
        expect(view.text()).toContain("Sign in with Microsoft");
        expect(view.text()).not.toContain("Where you play");
        expect(view.text()).not.toContain("Point Minecraft here");
    });

    it("shows the servers once signed in", async () => {
        const view = mount(ProxyPane, stub());
        await waitFor(() => expect(view.text()).toContain("Where you play"));
        expect(view.text()).toContain("Alaydriem's SMP");
    });

    // Every tile would be an offer that cannot be taken, so the tiles go entirely.
    it("replaces the servers with the reason when the server refuses a proxy", async () => {
        const view = mount(ProxyPane, stub({ capability: "disabled" }));
        await waitFor(() => expect(view.text()).toContain("will not accept a proxy"));
        expect(view.text()).not.toContain("Alaydriem's SMP");
    });

    // Unknown is not refused. The proxy can still be started; the callout says what will
    // happen if position is rejected.
    it("warns but still offers the servers when capability is unknown", async () => {
        const view = mount(ProxyPane, stub({ capability: "unknown" }));
        await waitFor(() => expect(view.text()).toContain("could not reach this server"));
        expect(view.text()).toContain("Alaydriem's SMP");
    });

    // Minecraft is usually on this machine, so the loopback address leads whatever else
    // the listener answers on.
    it("names the address to type into Minecraft", async () => {
        const view = mount(ProxyPane, stub());
        await waitFor(() => expect(view.text()).toContain("127.0.0.1:19132"));
    });

    // The listener binds every interface, so there is no choice to offer — and `0.0.0.0`
    // is not something anything can connect to. Every reachable address is listed instead.
    it("lists the addresses instead of offering a choice", async () => {
        const view = mount(ProxyPane, stub(), true);
        await waitFor(() => expect(view.text()).toContain("Point Minecraft at one of these"));
        expect(view.host.querySelector("select")).toBeNull();
        expect(view.text()).toContain("192.168.1.24:19132");
        // Minecraft is often on the same machine, so loopback is one of the options.
        expect(view.text()).toContain("127.0.0.1:19132");
        expect(view.text()).not.toContain("0.0.0.0:19132");
    });

    it("gives every listed address its own copy button", async () => {
        const view = mount(ProxyPane, stub(), true);
        await waitFor(() => expect(view.host.querySelector(".rad-address")).not.toBeNull());
        const rows = [...view.host.querySelectorAll(".rad-address")];
        expect(rows.length).toBeGreaterThan(1);
        expect(rows.every((r) => r.querySelector('[aria-label^="Copy"]'))).toBe(true);
    });

    // The listener has one mode and the picker never drove it, so a desktop reader is
    // offered the addresses it actually answers on rather than a choice with no effect.
    it("offers no interface choice on desktop either", async () => {
        const view = mount(ProxyPane, stub());
        await waitFor(() => expect(view.text()).toContain("Point Minecraft at one of these"));
        expect(view.host.querySelector("select")).toBeNull();
    });
});

describe("ProxyPane server list", () => {
    // PlateGrid only draws the add tile when it is given a handler. Without one there was no
    // way to add a backend at all, and nothing failed — the tile simply was not there.
    // The add tile lives in the grid, and the grid is not rendered when the list is empty.
    // So the one state that most needs the action was the only state without it.
    it("offers a way to add the first server when the list is empty", async () => {
        const empty = stub();
        (empty as unknown as { sortedProxyServers: Writable<unknown[]> }).sortedProxyServers =
            writable([]);
        const view = mount(ProxyPane, empty);
        await waitFor(() => expect(view.text()).toContain("No servers yet"));

        const add = [...view.host.querySelectorAll<HTMLElement>(".rad-btn")].find((b) =>
            b.textContent?.includes("Add a server"),
        );
        expect(add).not.toBeUndefined();
        add?.click();
        await waitFor(() => expect(view.host.querySelector(".rad-modal.is-open")).not.toBeNull());
    });

    it("offers a way to add a server", async () => {
        const view = mount(ProxyPane, stub());
        await waitFor(() => expect(view.text()).toContain("Where you play"));
        expect(view.host.querySelector(".rad-server-add")).not.toBeNull();
    });

    it("opens the editor from the add tile", async () => {
        const view = mount(ProxyPane, stub());
        await waitFor(() => expect(view.host.querySelector(".rad-server-add")).not.toBeNull());
        view.host.querySelector<HTMLElement>(".rad-server-add")?.click();
        await waitFor(() => expect(view.text()).toContain("Add a server"));
        expect(view.host.querySelector(".rad-modal.is-open")).not.toBeNull();
    });

    // An operator's entry comes from the server's config and cannot be edited locally, so
    // only the entries a reader owns offer it.
    it("offers edit only on an entry of your own", async () => {
        const view = mount(ProxyPane, stub());
        await waitFor(() => expect(view.text()).toContain("Alaydriem's SMP"));
        const plates = [...view.host.querySelectorAll(".rad-server")];
        expect(plates[0]?.querySelector('[aria-label^="Edit"]')).not.toBeNull();
        expect(plates[1]?.querySelector('[aria-label^="Edit"]')).toBeNull();
    });
});

// The server's own way in. A player whose device cannot stay on the network this client
// runs on has no local address that will work, and the relay is the answer — but only if
// the pane says so.
describe("Bedrock panes and the server's relay", () => {
    it("offers nothing extra when the server runs no relay", async () => {
        for (const pane of [ProxyPane, RealmsPane]) {
            const view = mount(pane, stub());
            await waitFor(() => expect(view.text()).toContain("Point Minecraft"));
            expect(view.text()).not.toContain("Or go through this BVC server");
        }
    });

    it("names the transfer relay on the BVC host", async () => {
        for (const pane of [ProxyPane, RealmsPane]) {
            const view = mount(pane, stub({ transferPort: 19132 }));
            await waitFor(() => expect(view.text()).toContain("Transfer server"));
            expect(view.text()).toContain("bvc.example.com:19132");
        }
    });

    it("adds the DNS override and what to do with it", async () => {
        for (const pane of [ProxyPane, RealmsPane]) {
            const view = mount(
                pane,
                stub({ transferPort: 19132, dnsOverrideHost: "geo.hivebedrock.network" }),
            );
            await waitFor(() => expect(view.text()).toContain("DNS override"));
            expect(view.text()).toContain("geo.hivebedrock.network");
            expect(view.text()).toContain("Point your device's DNS at bvc.example.com");
        }
    });

    it("gives each offered address its own copy button", async () => {
        const view = mount(
            ProxyPane,
            stub({ transferPort: 19132, dnsOverrideHost: "geo.hivebedrock.network" }),
        );
        await waitFor(() => expect(view.text()).toContain("DNS override"));
        const rows = [...view.host.querySelectorAll(".rad-address--offer")];
        expect(rows).toHaveLength(2);
        expect(rows.every((r) => r.querySelector('[aria-label^="Copy"]'))).toBe(true);
    });
});

describe("RealmsPane", () => {
    it("shows nothing but the sign-in until Microsoft is connected", () => {
        const view = mount(RealmsPane, stub({ authed: false }));
        expect(view.text()).toContain("Sign in with Microsoft");
        expect(view.text()).not.toContain("Your Realms");
    });

    it("lists the Realms once signed in", async () => {
        const view = mount(RealmsPane, stub());
        await waitFor(() => expect(view.text()).toContain("Your Realms"));
        expect(view.text()).toContain("Alaydriem's Realm");
    });

    // A closed Realm is still listed — it is yours — but it cannot be joined now.
    it("marks a closed Realm as closed", async () => {
        const view = mount(RealmsPane, stub());
        await waitFor(() => expect(view.text()).toContain("Hearthhold"));
        expect(view.text()).toContain("Closed");
    });

    it("replaces the Realms with the reason when the server refuses one", async () => {
        const view = mount(RealmsPane, stub({ capability: "disabled" }));
        await waitFor(() => expect(view.text()).toContain("will not accept a Realm"));
        expect(view.text()).not.toContain("Alaydriem's Realm");
    });
});
