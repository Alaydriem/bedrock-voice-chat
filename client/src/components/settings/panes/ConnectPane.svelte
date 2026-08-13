<script lang="ts">
  import { I18n } from "$lib/i18n";
    import { invoke } from "@tauri-apps/api/core";
    import { onDestroy, onMount } from "svelte";
    import Icon from "$radial/components/Icon.svelte";
    import type { BedrockManager } from "../../../js/app/managers/bedrock/BedrockManager";
    import type { BedrockCapabilityStatus } from "../../../js/app/managers/bedrock/BedrockCapabilityManager";
    import type { AddonMode } from "../../../js/bindings/AddonMode";
    import type { ProxyServerEntry } from "../../../js/app/managers/bedrock/ProxyServerEntry";
    import type { BedrockStatus } from "../../../js/bindings/BedrockStatus";
    import type { RealmEntry } from "../../../js/bindings/RealmEntry";
    import type { ProtocolVersionOption } from "../../../js/bindings/ProtocolVersionOption";
    import type { ListState } from "../../../js/app/settings/ListState";
    import type { Plate } from "../../../js/app/settings/Plate";
    import Loader from "$radial/components/Loader.svelte";
    import ListShell from "../ListShell.svelte";
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
    let capability = $state<BedrockCapabilityStatus | null>(null);
    let checking = $state(false);
    let servers = $state<readonly ProxyServerEntry[]>([]);
    let serverFavourites = $state<ReadonlySet<string>>(new Set());
    let activeId = $state<string | null>(null);
    let running = $state(false);
    let realms = $state<readonly RealmEntry[]>([]);
    let loadingRealms = $state(false);
    let realmFavourites = $state<ReadonlySet<string>>(new Set());
    let activeRealmId = $state<bigint | null>(null);
    let activeRealmName = $state("");
    let startedAt = $state<number | null>(null);
    let now = $state(Math.floor(Date.now() / 1000));

    // The one thing that differs between two saved worlds that a reader can act on.
    // An address would say less and is what this pane exists to stop showing.
    function sends(mode: AddonMode | undefined): string {
        return mode === "net"
            ? I18n.t("BVC Addon Required")
            : I18n.t("Realms / No-Net Addon Required");
    }

    const offered = $derived(servers.filter((entry) => entry.source === "server"));
    const owned = $derived(servers.filter((entry) => entry.source !== "server"));

    function plate(entry: ProxyServerEntry): Plate {
        const live = entry.id === activeId && running;
        return {
            id: entry.id,
            name: entry.name,
            detail: sends(entry.addonMode),
            glyphKey: entry.host,
            chips: live ? ([{ label: "Forwarding here", severity: "ok" }] as const) : [],
            favourite: serverFavourites.has(entry.id),
            active: live,
            reachable: capability !== "disabled",
            readonly: entry.source === "server",
        };
    }

    const offeredPlates = $derived<readonly Plate[]>(offered.map(plate));
    const ownedPlates = $derived<readonly Plate[]>(owned.map(plate));

    const realmPlates = $derived<readonly Plate[]>(
        realms.map((realm) => {
            const open = realm.state?.toLowerCase() === "open";
            return {
                id: String(realm.id),
                name: realm.name,
                detail: I18n.t("Realm") + " - " + (realm.motd || "Minecraft Bedrock"),
                glyphKey: `${realm.name}-realm`,
                chips: [
                    ...(realm.id === activeRealmId
                        ? ([{ label: "Forwarding here", severity: "ok" }] as const)
                        : []),
                    ...(open ? [] : ([{ label: "Closed", severity: "muted" }] as const)),
                ],
                favourite: realmFavourites.has(String(realm.id)),
                active: realm.id === activeRealmId,
                reachable: open && capability !== "disabled",
                readonly: true,
            };
        }),
    );

    // One shell over all three sections. Capability refused outranks any list that
    // loaded: every row would be an offer that cannot be taken.
    const listState = $derived<ListState>(
        capability === "disabled" ? "failed" : checking || loadingRealms ? "loading" : "ready",
    );
    const rows = $derived(realmPlates.length + offeredPlates.length + ownedPlates.length);

    // A Realm carries its own name; a direct session is named by the entry it forwards to.
    const activeName = $derived(
        activeRealmId !== null
            ? activeRealmName
            : running
              ? (servers.find((entry) => entry.id === activeId)?.name ?? "")
              : "",
    );

    const unsubs: Array<() => void> = [];
    let tick: ReturnType<typeof setInterval> | null = null;

    // `proxyRunning` is the manager's own view. This is the backend's, and the
    // connected instruction is the one thing on the pane that must not be stale.
    async function pollStatus(): Promise<void> {
        try {
            const status = await invoke<BedrockStatus>("bedrock_get_status");
            running = status.proxy_running;
            startedAt = status.proxy_started_at === null ? null : Number(status.proxy_started_at);
        } catch {
            running = false;
        }
    }

    function clock(seconds: number): string {
        const safe = Math.max(0, seconds);
        const parts = [Math.floor(safe / 3600), Math.floor((safe % 3600) / 60), safe % 60];
        return parts.map((n) => String(n).padStart(2, "0")).join(":");
    }

    // Only a direct session reports a start time, so a Realm shows the world
    // without one rather than a clock counting from zero.
    const uptime = $derived(startedAt === null ? "" : clock(now - startedAt));

    onMount(() => {
        unsubs.push(bedrock.isAuthenticated.subscribe((v) => (authed = v)));
        unsubs.push(bedrock.isRestoringAuth.subscribe((v) => (checkingAuth = v)));
        unsubs.push(bedrock.capability.status.subscribe((v) => (capability = v)));
        unsubs.push(bedrock.capability.isChecking.subscribe((v) => (checking = v)));
        unsubs.push(bedrock.sortedProxyServers.subscribe((v) => (servers = v)));
        unsubs.push(bedrock.proxyFavorites.subscribe((v) => (serverFavourites = v)));
        unsubs.push(bedrock.activeProxyId.subscribe((v) => (activeId = v)));
        unsubs.push(bedrock.proxyRunning.subscribe((v) => (running = v)));
        unsubs.push(bedrock.sortedRealms.subscribe((v) => (realms = v)));
        unsubs.push(bedrock.isLoadingRealms.subscribe((v) => (loadingRealms = v)));
        unsubs.push(bedrock.favorites.subscribe((v) => (realmFavourites = v)));
        unsubs.push(bedrock.activeRealmId.subscribe((v) => (activeRealmId = v)));
        unsubs.push(bedrock.activeRealmName.subscribe((v) => (activeRealmName = v)));

        void pollStatus();
        void bedrock.initializeRealmsAccess();
        void bedrock
            .listProtocolVersions()
            .then((v) => (versions = v))
            .catch(() => {});

        tick = setInterval(() => {
            now = Math.floor(Date.now() / 1000);
            void pollStatus();
        }, 1_000);
    });

    onDestroy(() => {
        for (const off of unsubs) off();
        if (tick) clearInterval(tick);
    });

    function connectServer(id: string): void {
        const entry = servers.find((s) => s.id === id);
        if (entry) void bedrock.connectToProxyServer(entry);
    }

    function connectRealm(id: string): void {
        const realm = realms.find((r) => String(r.id) === id);
        if (realm) void bedrock.connectToRealm(realm);
    }

    /** An entry to change, null to add, undefined for closed. */
    let editing = $state<ProxyServerEntry | null | undefined>(undefined);
    let versions = $state<readonly ProtocolVersionOption[]>([]);

    async function saveServer(
        name: string,
        host: string,
        port: number,
        protocolVersion: number | undefined,
        addonMode: AddonMode,
        id?: string,
    ): Promise<void> {
        editing = undefined;
        if (id)
            await bedrock.updateProxyServer(id, { name, host, port, protocolVersion, addonMode });
        else await bedrock.addProxyServer(name, host, port, protocolVersion, addonMode);
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
                    <span class="rad-account__meta">{I18n.t("BVC JOINS THE WORLD AS YOU")}</span>
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
                    {I18n.t("You can connect anyway — if position is refused, this is why.")}
                    <button class="rad-btn rad-btn--quiet" onclick={() => void bedrock.capability.refresh()}>
                        {I18n.t("Check again")}
                    </button>
                </span>
            </div>
        {/if}

        <ListShell
            state={listState}
            count={rows}
            failTitle="This server will not accept a proxy"
            failNote="Bedrock support is turned off here, so position sent from a proxy is discarded. Ask the operator to turn it on, or switch to a server that has it."
            retryLabel="Check again"
            onretry={() => void bedrock.capability.refresh()}
            emptyTitle="No servers yet"
            emptyNote="Add the world you would have joined in Minecraft, and BVC will sit in front of it."
        >
            {#snippet emptyAction()}
                <button class="rad-btn rad-btn--primary" onclick={() => (editing = null)}>
                    <Icon name="plus" /> {I18n.t("Add a server")}
                </button>
            {/snippet}

            <!-- Read-only, and never carrying an addon-mode control: `new_realm` is always
                 no-net, so a Realm has no mode to declare. -->
            {#if realmPlates.length > 0}
                <div class="rad-section__head" style="font-size: var(--text-rad-lead)">
                    {I18n.t("Realms")}
                </div>
                <PlateGrid
                    plates={realmPlates}
                    onconnect={connectRealm}
                    onstop={() => void bedrock.stopRealms()}
                    onfavourite={(id) => void bedrock.toggleFavorite(BigInt(id))}
                />
            {/if}

            <!-- The operator's list. Read-only here, because the server's word is final on
                 an entry it advertises. -->
            {#if offeredPlates.length > 0}
                <div class="rad-section__head" style="font-size: var(--text-rad-lead)">
                    {I18n.t("From your server")}
                </div>
                <PlateGrid
                    plates={offeredPlates}
                    onconnect={connectServer}
                    onstop={() => void bedrock.stopProxy()}
                    onfavourite={(id) => void bedrock.toggleProxyFavorite(id)}
                />
            {/if}

            <!-- Always rendered: this section carries the way to add an entry, and hiding it
                 when empty is what once left a reader with a list they could not add to. -->
            <div class="rad-section__head" style="font-size: var(--text-rad-lead)">
                {I18n.t("Yours")}
            </div>
            <PlateGrid
                plates={ownedPlates}
                addLabel="Add a server"
                onconnect={connectServer}
                onstop={() => void bedrock.stopProxy()}
                onfavourite={(id) => void bedrock.toggleProxyFavorite(id)}
                onadd={() => (editing = null)}
                onedit={(id) => (editing = servers.find((s) => s.id === id) ?? null)}
                onremove={(id) => void bedrock.deleteProxyServer(id)}
            />
        </ListShell>



        {#if activeName}
            <div class="rad-callout">
                <span>
                    {I18n.t("Connected")} — <b>{activeName}</b>{#if uptime}
                        · {I18n.tf("Running for {clock}", { clock: uptime })}
                    {/if}<br />
                    {I18n.t("Open Minecraft, then join it from the Friends tab.")}
                </span>
            </div>
        {/if}

        <BedrockLog {bedrock} {mobile} live={running || activeRealmId !== null} />
    {/if}
</div>

<ProxyServerEditor
    entry={editing}
    {versions}
    onsave={(name, host, port, protocolVersion, addonMode, id) =>
        void saveServer(name, host, port, protocolVersion, addonMode, id)}
    oncancel={() => (editing = undefined)}
/>

<XboxSignIn {bedrock} />
