import { render, waitFor } from "@testing-library/svelte";
import { readable, writable, type Writable } from "svelte/store";
import { beforeEach, describe, expect, it } from "vitest";
import { mockInvoke } from "../tauri";
import type { BedrockManager } from "../../js/app/managers/bedrock/BedrockManager";
import type { BedrockCapabilityStatus } from "../../js/app/managers/bedrock/BedrockCapabilityManager";

const { default: ConnectPane } = await import("../../components/settings/panes/ConnectPane.svelte");
const { default: ProxyServerEditor } = await import("../../components/settings/ProxyServerEditor.svelte");

interface Knobs {
    authed?: boolean;
    capability?: BedrockCapabilityStatus | null;
    restoring?: boolean;
    /** Names the world the session is forwarding to. Empty means nothing is running. */
    activeName?: string;
}

/**
 * A stand-in for the Bedrock manager.
 *
 * The real one opens an Xbox session on construction. What these tests are about is
 * which surface the pane shows for a given answer, so the answers are supplied directly.
 */
function stub({
    authed = true,
    capability = "enabled",
    restoring = false,
    activeName = "",
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
            refresh: async () => {},
        },
        sortedProxyServers: servers,
        proxyFavorites: readable(new Set<string>()),
        activeProxyId: readable<string | null>(null),
        proxyRunning: readable(false),
        sortedRealms: realms,
        isLoadingRealms: readable(false),
        favorites: readable(new Set<string>()),
        // The connected callout reads the active session's name. A Realm is the one
        // backend whose name arrives on its own store, so the knob rides it.
        activeRealmId: readable<bigint | null>(activeName ? 1n : null),
        activeRealmName: readable(activeName),
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
        listProtocolVersions: async () => [{ protocol: 800, label: "1.21.100" }],
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
    return {
        host,
        text: () => host.textContent ?? "",
        /**
         * The section headings alone. A row's own copy can repeat a heading's
         * wording, so "is that section present" has to ask the headings.
         */
        heads: () =>
            [...host.querySelectorAll(".rad-section__head")].map((h) => h.textContent?.trim()),
        /** The connected callout, never the capability warning beside it. */
        callout: () =>
            host.querySelector(".rad-callout:not(.rad-callout--warn)")?.textContent ?? "",
    };
}

function status(over: Record<string, unknown> = {}) {
    return {
        proxy_running: false,
        realms_running: false,
        xbox_authenticated: true,
        proxy_target_host: null,
        proxy_target_port: null,
        proxy_listen_port: 28282,
        active_realm_id: null,
        active_realm_name: null,
        proxy_started_at: null,
        ...over,
    };
}

beforeEach(() => {
    mockInvoke({ bedrock_get_status: () => status() });
});

describe("Connect pane", () => {
    it("renders all three sections when each has rows", async () => {
        const view = mount(ConnectPane, stub());
        await waitFor(() => expect(view.heads()).toHaveLength(3));
        expect(view.heads()).toEqual(["Realms", "From your server", "Yours"]);
    });

    // The old flow's entire failure mode was telling the user about ports.
    it("shows no address or port anywhere", async () => {
        const view = mount(ConnectPane, stub());
        await waitFor(() => expect(view.text()).toContain("Realms"));
        expect(view.text()).not.toMatch(/\d+\.\d+\.\d+\.\d+/);
        expect(view.text()).not.toContain("28282");
        expect(view.text()).not.toContain(":19132");
    });

    it("names the world in the connected state", async () => {
        const view = mount(ConnectPane, stub({ activeName: "Truly Bedrock SMP" }));
        await waitFor(() => expect(view.text()).toContain("Truly Bedrock SMP"));
        expect(view.text()).toContain("Friends");
    });

    // How long the session has been up is the one number worth printing: it is what
    // distinguishes "connected" from "connected a moment ago and silently dropped".
    it("counts the session's uptime beside the world", async () => {
        const started = Math.floor(Date.now() / 1000) - 65;
        mockInvoke({
            bedrock_get_status: () =>
                status({ proxy_running: true, proxy_started_at: BigInt(started) }),
        });
        const view = mount(ConnectPane, stub({ activeName: "Truly Bedrock SMP" }));
        await waitFor(() => expect(view.callout()).toMatch(/00:01:0\d/));
    });

    // A Realm reports no start time, so the world is named without a clock counting
    // up from zero.
    it("omits the clock when nothing reports a start time", async () => {
        const view = mount(ConnectPane, stub({ activeName: "Truly Bedrock SMP" }));
        await waitFor(() => expect(view.callout()).toContain("Truly Bedrock SMP"));
        expect(view.callout()).not.toMatch(/\d\d:\d\d:\d\d/);
    });

    // Nothing is running, so an instruction to go to the Friends tab would name no world.
    it("says nothing about the Friends tab while stopped", async () => {
        const view = mount(ConnectPane, stub());
        await waitFor(() => expect(view.text()).toContain("Realms"));
        expect(view.text()).not.toContain("Friends");
    });

    // A section with no rows is a heading over nothing. Only "Yours" persists, because
    // it carries the way to add the first entry.
    it("hides the read-only sections when they have no rows", async () => {
        const bare = stub();
        (bare as unknown as { sortedRealms: Writable<unknown[]> }).sortedRealms = writable([]);
        (bare as unknown as { sortedProxyServers: Writable<unknown[]> }).sortedProxyServers =
            writable([{ id: "b1", name: "Alaydriem's SMP", host: "mc.alaydriem.com", port: 19132 }]);

        const view = mount(ConnectPane, bare);
        await waitFor(() => expect(view.text()).toContain("Alaydriem's SMP"));
        expect(view.heads()).toEqual(["Yours"]);
    });
});

describe("Connect pane authentication gate", () => {
    // The proxy joins the backend as you. Everything below the gate is meaningless
    // without an account, and showing it invites somebody to configure a dead end.
    it("shows nothing but the sign-in until Microsoft is connected", () => {
        const view = mount(ConnectPane, stub({ authed: false }));
        expect(view.text()).toContain("Sign in with Microsoft");
        expect(view.text()).not.toContain("Yours");
    });

    // A stored session is read back on start, so `authed` is false for a moment before it
    // is true. Taking that moment as an answer showed a sign-in card that swapped itself out.
    it("shows a loader rather than a sign-in card while the session is read back", () => {
        const view = mount(ConnectPane, stub({ authed: false, restoring: true }));
        expect(view.text()).not.toContain("Sign in with Microsoft");
        expect(view.host.querySelector(".rad-loader")).not.toBeNull();
    });

    it("shows the sign-in card once the read back finds nothing", () => {
        const view = mount(ConnectPane, stub({ authed: false, restoring: false }));
        expect(view.text()).toContain("Sign in with Microsoft");
    });
});

describe("Connect pane rows", () => {
    it("lists Realms and servers once signed in", async () => {
        const view = mount(ConnectPane, stub());
        await waitFor(() => expect(view.text()).toContain("Alaydriem's Realm"));
        expect(view.text()).toContain("Alaydriem's SMP");
    });

    // A closed Realm is still listed — it is yours — but it cannot be joined now.
    it("marks a closed Realm as closed", async () => {
        const view = mount(ConnectPane, stub());
        await waitFor(() => expect(view.text()).toContain("Alaydriem's Realm"));
        expect(view.text()).toContain("Closed");
    });

    // Every tile would be an offer that cannot be taken, so the tiles go entirely.
    it("replaces every row with the reason when the server refuses a proxy", async () => {
        const view = mount(ConnectPane, stub({ capability: "disabled" }));
        await waitFor(() => expect(view.text()).toContain("will not accept a proxy"));
        expect(view.text()).not.toContain("Alaydriem's SMP");
        expect(view.text()).not.toContain("Alaydriem's Realm");
    });

    // Unknown is not refused. The proxy can still be started; the callout says what will
    // happen if position is rejected.
    it("warns but still offers the rows when capability is unknown", async () => {
        const view = mount(ConnectPane, stub({ capability: "unknown" }));
        await waitFor(() => expect(view.text()).toContain("could not reach this server"));
        expect(view.text()).toContain("Alaydriem's SMP");
    });

    // An operator's entry comes from the server's config and cannot be edited locally, so
    // only the entries a reader owns offer it. Grids render Realms, then the server's, then
    // the reader's own.
    it("offers edit only in the section of your own entries", async () => {
        const view = mount(ConnectPane, stub());
        await waitFor(() => expect(view.text()).toContain("Alaydriem's SMP"));
        const grids = [...view.host.querySelectorAll(".rad-server-grid")];
        expect(grids).toHaveLength(3);
        expect(grids[0]?.querySelector('[aria-label^="Edit"]')).toBeNull();
        expect(grids[1]?.querySelector('[aria-label^="Edit"]')).toBeNull();
        expect(grids[2]?.querySelector('[aria-label^="Edit"]')).not.toBeNull();
    });
});

describe("Connect pane add affordance", () => {
    // The add tile lives in the grid, and the grid was not rendered when the list was
    // empty. So the one state that most needs the action was the only state without it.
    it("offers a way to add the first server when nothing is saved", async () => {
        const empty = stub();
        (empty as unknown as { sortedRealms: Writable<unknown[]> }).sortedRealms = writable([]);
        (empty as unknown as { sortedProxyServers: Writable<unknown[]> }).sortedProxyServers =
            writable([]);
        const view = mount(ConnectPane, empty);
        await waitFor(() => expect(view.text()).toContain("No servers yet"));

        const add = [...view.host.querySelectorAll<HTMLElement>(".rad-btn")].find((b) =>
            b.textContent?.includes("Add a server"),
        );
        expect(add).not.toBeUndefined();
        add?.click();
        await waitFor(() => expect(view.host.querySelector(".rad-modal.is-open")).not.toBeNull());
    });

    it("opens the editor from the add tile", async () => {
        const view = mount(ConnectPane, stub());
        await waitFor(() => expect(view.host.querySelector(".rad-server-add")).not.toBeNull());
        view.host.querySelector<HTMLElement>(".rad-server-add")?.click();
        await waitFor(() => expect(view.host.querySelector(".rad-modal.is-open")).not.toBeNull());
    });
});

describe("Connect pane log", () => {
    // One omission explained three reports: signed-in state forgotten, favourites forgotten,
    // and an empty server list. `initialize` is what loads both managers from the store and
    // restores the Microsoft session, and the panes never called it.
    it("shows the log panel, which only exists once the manager is loaded", async () => {
        const view = mount(ConnectPane, stub());
        await waitFor(() => expect(view.text()).toContain("Connection log"));
    });

    // A log is the longest thing on the pane. Open by default on a phone it puts every
    // control above it behind a scroll.
    it("collapses the log by default on mobile and opens it on desktop", async () => {
        const phone = mount(ConnectPane, stub(), true);
        await waitFor(() => expect(phone.host.querySelector(".rad-disclosure")).not.toBeNull());
        expect(phone.host.querySelector(".rad-disclosure")?.classList.contains("is-open")).toBe(false);

        const desk = mount(ConnectPane, stub());
        await waitFor(() => expect(desk.host.querySelector(".rad-disclosure")).not.toBeNull());
        expect(desk.host.querySelector(".rad-disclosure")?.classList.contains("is-open")).toBe(true);
    });
});

describe("Proxy server editor addon mode", () => {
    function mountEditor(entry: unknown) {
        const host = document.createElement("div");
        document.body.append(host);
        render(ProxyServerEditor as never, {
            target: host,
            props: { entry, versions: [], onsave: () => {}, oncancel: () => {} },
        } as never);
        return host.querySelector('input[type="checkbox"]') as HTMLInputElement | null;
    }

    // The server's word is final on an entry it advertises, so the control shows
    // the declaration without offering to change it.
    it("greys out the toggle on an advertised entry", () => {
        const box = mountEditor({
            id: "server:play.example.com:19132",
            name: "Advertised",
            host: "play.example.com",
            port: 19132,
            addonMode: "no_net",
            source: "server",
        });
        expect(box).not.toBeNull();
        expect(box!.checked).toBe(true);
        expect(box!.disabled).toBe(true);
    });

    it("leaves the toggle editable on a user-created entry", () => {
        const box = mountEditor({
            id: "local:1",
            name: "Mine",
            host: "play.example.com",
            port: 19132,
            addonMode: "net",
        });
        expect(box).not.toBeNull();
        expect(box!.checked).toBe(false);
        expect(box!.disabled).toBe(false);
    });
});
