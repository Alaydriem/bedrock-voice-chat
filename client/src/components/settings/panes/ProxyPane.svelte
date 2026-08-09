<script lang="ts">
  import { I18n } from "$lib/i18n";
    import { invoke } from "@tauri-apps/api/core";
    import { onDestroy, onMount } from "svelte";
    import Icon from "$radial/components/Icon.svelte";
    import SettingRow from "$radial/components/SettingRow.svelte";
    import StatusChip from "$radial/components/StatusChip.svelte";
    import type { BedrockManager } from "../../../js/app/managers/bedrock/BedrockManager";
    import type { BedrockCapabilityStatus } from "../../../js/app/managers/bedrock/BedrockCapabilityManager";
    import type { ProxyServerEntry } from "../../../js/app/managers/bedrock/ProxyServerEntry";
    import type { BedrockStatus } from "../../../js/bindings/BedrockStatus";
    import type { NetworkInterface } from "../../../js/bindings/NetworkInterface";
    import type { ProtocolVersionOption } from "../../../js/bindings/ProtocolVersionOption";
    import type { ListState } from "../../../js/app/settings/ListState";
    import type { Plate } from "../../../js/app/settings/Plate";
    import { ListenAddress } from "../../../js/app/settings/ListenAddress";
    import { BedrockRelayAddresses } from "../../../js/app/settings/BedrockRelayAddresses";
    import Loader from "$radial/components/Loader.svelte";
    import ListShell from "../ListShell.svelte";
    import ListenAddresses from "../ListenAddresses.svelte";
    import RelayAddresses from "../RelayAddresses.svelte";
    import BedrockLog from "../BedrockLog.svelte";
    import ProxyServerEditor from "../ProxyServerEditor.svelte";
    import XboxSignIn from "../XboxSignIn.svelte";
    import PlateGrid from "../PlateGrid.svelte";

    interface Props {
        bedrock: BedrockManager;
        mobile?: boolean;
    }
    let { bedrock, mobile = false }: Props = $props();

    let authed = $state(false);
    // Starts true on the manager, so the answer is never guessed at before it arrives.
    let checkingAuth = $state(true);
    let serverHost = $state("");
    let transferPort = $state<number | null>(null);
    let dnsOverrideHost = $state<string | null>(null);
    let capability = $state<BedrockCapabilityStatus | null>(null);
    let checking = $state(false);
    let servers = $state<readonly ProxyServerEntry[]>([]);
    let favourites = $state<ReadonlySet<string>>(new Set());
    let activeId = $state<string | null>(null);
    let running = $state(false);
    let interfaces = $state<readonly NetworkInterface[]>([]);
    let listenPort = $state(19132);
    let startedAt = $state<number | null>(null);
    let now = $state(Math.floor(Date.now() / 1000));

    const relayAddresses = $derived(
        BedrockRelayAddresses.list({ host: serverHost, transferPort, dnsOverrideHost }),
    );
    // The listener binds every interface on every platform — `bedrock::proxy::manager`
    // has no other mode — so there is nothing here to choose. The picker that stood here
    // changed only which address this pane suggested typing into Minecraft, which made a
    // narrower bind look selectable when none was ever applied.
    const join = $derived(ListenAddress.join(ListenAddress.ANY, listenPort));

    // Capability refused outranks a list that loaded.
    const listState = $derived<ListState>(
        capability === "disabled" ? "failed" : checking ? "loading" : "ready",
    );

    const plates = $derived<readonly Plate[]>(
        servers.map((entry) => ({
            id: entry.id,
            name: entry.name,
            detail: `${entry.host}:${entry.port}`,
            glyphKey: entry.host,
            chips: [
                ...(entry.id === activeId && running
                    ? ([{ label: "Forwarding here", severity: "ok" }] as const)
                    : []),
                ...(entry.source === "server"
                    ? ([{ label: "From your server", severity: "muted" }] as const)
                    : []),
            ],
            favourite: favourites.has(entry.id),
            active: entry.id === activeId && running,
            reachable: capability !== "disabled",
            readonly: entry.source === "server",
        })),
    );

    const unsubs: Array<() => void> = [];
    let tick: ReturnType<typeof setInterval> | null = null;

    async function pollStatus(): Promise<void> {
        try {
            const status = await invoke<BedrockStatus>("bedrock_get_status");
            running = status.proxy_running;
            startedAt = status.proxy_started_at === null ? null : Number(status.proxy_started_at);
            if (status.proxy_listen_port) listenPort = status.proxy_listen_port;
        } catch {
            running = false;
        }
    }

    onMount(() => {
        unsubs.push(bedrock.isAuthenticated.subscribe((v) => (authed = v)));
        unsubs.push(bedrock.isRestoringAuth.subscribe((v) => (checkingAuth = v)));
        unsubs.push(bedrock.capability.serverHost.subscribe((v) => (serverHost = v)));
        unsubs.push(bedrock.capability.transferPort.subscribe((v) => (transferPort = v)));
        unsubs.push(bedrock.capability.dnsOverrideHost.subscribe((v) => (dnsOverrideHost = v)));
        unsubs.push(bedrock.capability.status.subscribe((v) => (capability = v)));
        unsubs.push(bedrock.capability.isChecking.subscribe((v) => (checking = v)));
        unsubs.push(bedrock.sortedProxyServers.subscribe((v) => (servers = v)));
        unsubs.push(bedrock.proxyFavorites.subscribe((v) => (favourites = v)));
        unsubs.push(bedrock.activeProxyId.subscribe((v) => (activeId = v)));
        unsubs.push(bedrock.proxyRunning.subscribe((v) => (running = v)));
        unsubs.push(bedrock.interfaces.subscribe((v) => (interfaces = v)));
        unsubs.push(bedrock.listenPort.subscribe((v) => (listenPort = v)));

        void bedrock.loadInterfaces();
        void pollStatus();
        void bedrock.listProtocolVersions().then((v) => (versions = v)).catch(() => {});

        tick = setInterval(() => {
            now = Math.floor(Date.now() / 1000);
            void pollStatus();
        }, 1_000);
    });

    onDestroy(() => {
        for (const off of unsubs) off();
        if (tick) clearInterval(tick);
    });

    function clock(seconds: number): string {
        const safe = Math.max(0, seconds);
        const parts = [Math.floor(safe / 3600), Math.floor((safe % 3600) / 60), safe % 60];
        return parts.map((n) => String(n).padStart(2, "0")).join(":");
    }

    async function copy(text: string): Promise<void> {
        await navigator.clipboard?.writeText(text).catch(() => {});
    }

    function connect(id: string): void {
        const entry = servers.find((s) => s.id === id);
        if (entry) void bedrock.connectToProxyServer(entry);
    }

    /** An entry to change, null to add, undefined for closed. */
    let editing = $state<ProxyServerEntry | null | undefined>(undefined);
    let versions = $state<readonly ProtocolVersionOption[]>([]);

    async function saveServer(
        name: string,
        host: string,
        port: number,
        protocolVersion: number | undefined,
        id?: string,
    ): Promise<void> {
        editing = undefined;
        if (id) await bedrock.updateProxyServer(id, { name, host, port, protocolVersion });
        else await bedrock.addProxyServer(name, host, port, protocolVersion);
    }
</script>

<div class="rad-section">
    {#if checkingAuth}
        <div class="rad-card">
            <div class="rad-empty" style="padding: 34px 20px">
                <Loader loading size={72} />
                <span class="rad-empty__note">{I18n.t("Checking your Microsoft sign-in.")}</span>
            </div>
        </div>
    {:else if !authed}
        <div class="rad-card">
            <div class="rad-account rad-account--stack">
                <span class="rad-account__badge" style="background: #107c10">MS</span>
                <span class="rad-account__text">
                    <span class="rad-account__name">{I18n.t("Sign in with Microsoft")}</span>
                    <span class="rad-account__meta">{I18n.t("THE PROXY JOINS THE SERVER AS YOU")}</span>
                </span>
                <button class="rad-btn rad-btn--primary" onclick={() => void bedrock.openLoginModal()}>
                    {I18n.t("Sign in")}
                </button>
            </div>
        </div>
    {:else}
        {#if capability === "unknown"}
            <div class="rad-callout rad-callout--warn">
                <span>
                    <b>{I18n.t("We could not reach this server to ask whether Bedrock support is on.")}</b>
                    {I18n.t("You can start the proxy anyway — if position is refused, this is why.")}
                    <button class="rad-btn rad-btn--quiet" onclick={() => void bedrock.capability.refresh()}>
                        {I18n.t("Check again")}
                    </button>
                </span>
            </div>
        {/if}

        <div class="rad-section__head" style="font-size: var(--text-rad-lead)">{I18n.t("Where you play")}</div>
        <br />
        <ListShell
            state={listState}
            count={plates.length}
            failTitle="This server will not accept a proxy"
            failNote="Bedrock support is turned off here, so position sent from a proxy is discarded. Ask the operator to turn it on, or switch to a server that has it."
            retryLabel="Check again"
            onretry={() => void bedrock.capability.refresh()}
            emptyTitle="No servers yet"
            emptyNote="Add the address you would have joined in Minecraft, and the proxy will sit in front of it."
        >
            {#snippet emptyAction()}
                <button class="rad-btn rad-btn--primary" onclick={() => (editing = null)}>
                    <Icon name="plus" /> {I18n.t("Add a server")}
                </button>
            {/snippet}
            <PlateGrid
                {plates}
                addLabel="Add a server"
                onconnect={connect}
                onstop={() => void bedrock.stopProxy()}
                onfavourite={(id) => void bedrock.toggleProxyFavorite(id)}
                onadd={() => (editing = null)}
                onedit={(id) => (editing = servers.find((s) => s.id === id) ?? null)}
                onremove={(id) => void bedrock.deleteProxyServer(id)}
            />
        </ListShell>

        <div class="rad-card">
            <ListenAddresses
                {interfaces}
                port={listenPort}
                bind={ListenAddress.ANY}
                singleLabel="Point Minecraft here"
                singleNote="Add this as a server in Minecraft and join it instead."
                listLabel="Point Minecraft at one of these"
                listNote="The proxy answers on all of them. Use the one for the device you play on."
                includeLoopback
                empty="No network address yet. Connect to Wi-Fi and reopen this pane."
            />

            <RelayAddresses addresses={relayAddresses} />

            <SettingRow
                label={I18n.t("Status")}
                note={running && startedAt
                    ? `Running for ${clock(now - startedAt)}`
                    : "Pick where you play, then connect."}
            >
                {#snippet control()}
                    <StatusChip severity={running ? "ok" : "muted"}>
                        {running ? "Listening" : "Stopped"}
                    </StatusChip>
                    {#if running}
                        <button class="rad-btn rad-btn--danger" onclick={() => void bedrock.stopProxy()}>
                            <Icon name="stop" /> {I18n.t("Stop")}
                        </button>
                    {/if}
                {/snippet}
            </SettingRow>
        </div>

        <div class="rad-callout rad-callout--warn">
            <span>
                {I18n.t("Join")} <b>{join}</b> in Minecraft, not the server itself. Joining directly skips the
                proxy and voice stays non-positional.
            </span>
        </div>

        <BedrockLog {bedrock} {mobile} live={running} />
    {/if}
</div>

<ProxyServerEditor
    entry={editing}
    {versions}
    onsave={(name, host, port, protocolVersion, id) =>
        void saveServer(name, host, port, protocolVersion, id)}
    oncancel={() => (editing = undefined)}
/>

<XboxSignIn {bedrock} />
