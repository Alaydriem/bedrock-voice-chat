<script lang="ts">
    import { onDestroy, onMount } from "svelte";
    import Icon from "$radial/components/Icon.svelte";
    import SettingRow from "$radial/components/SettingRow.svelte";
    import StatusChip from "$radial/components/StatusChip.svelte";
    import type { BedrockManager } from "../../../js/app/managers/bedrock/BedrockManager";
    import type { BedrockCapabilityStatus } from "../../../js/app/managers/bedrock/BedrockCapabilityManager";
    import type { NetworkInterface } from "../../../js/bindings/NetworkInterface";
    import type { RealmEntry } from "../../../js/bindings/RealmEntry";
    import type { ListState } from "../../../js/app/settings/ListState";
    import type { Plate } from "../../../js/app/settings/Plate";
    import { ListenAddress } from "../../../js/app/settings/ListenAddress";
    import { BedrockRelayAddresses } from "../../../js/app/settings/BedrockRelayAddresses";
    import Loader from "$radial/components/Loader.svelte";
    import ListShell from "../ListShell.svelte";
    import ListenAddresses from "../ListenAddresses.svelte";
    import RelayAddresses from "../RelayAddresses.svelte";
    import BedrockLog from "../BedrockLog.svelte";
    import XboxSignIn from "../XboxSignIn.svelte";
    import PlateGrid from "../PlateGrid.svelte";

    interface Props {
        bedrock: BedrockManager;
        mobile?: boolean;
    }
    let { bedrock, mobile = false }: Props = $props();

    const LISTEN_PORT = 19132;

    let authed = $state(false);
    // Starts true on the manager, so the answer is never guessed at before it arrives.
    let checkingAuth = $state(true);
    let serverHost = $state("");
    let transferPort = $state<number | null>(null);
    let dnsOverrideHost = $state<string | null>(null);
    let capability = $state<BedrockCapabilityStatus | null>(null);
    let realms = $state<readonly RealmEntry[]>([]);
    let loading = $state(false);
    let favourites = $state<ReadonlySet<string>>(new Set());
    let activeId = $state<bigint | null>(null);
    let activeName = $state("");
    let interfaces = $state<readonly NetworkInterface[]>([]);
    let bind = $state(ListenAddress.LOOPBACK);

    const relayAddresses = $derived(
        BedrockRelayAddresses.list({ host: serverHost, transferPort, dnsOverrideHost }),
    );
    const effectiveBind = $derived(mobile ? ListenAddress.ANY : bind);
    const join = $derived(ListenAddress.join(effectiveBind, LISTEN_PORT));
    const choices = $derived(ListenAddress.choices(interfaces));

    const listState = $derived<ListState>(
        capability === "disabled" ? "failed" : loading ? "loading" : "ready",
    );

    const plates = $derived<readonly Plate[]>(
        realms.map((realm) => {
            const open = realm.state?.toLowerCase() === "open";
            return {
                id: String(realm.id),
                name: realm.name,
                detail: realm.motd || "No description",
                glyphKey: `${realm.name}-realm`,
                chips: [
                    ...(realm.id === activeId
                        ? ([{ label: "Forwarding here", severity: "ok" }] as const)
                        : []),
                    ...(open ? [] : ([{ label: "Closed", severity: "muted" }] as const)),
                ],
                favourite: favourites.has(String(realm.id)),
                active: realm.id === activeId,
                reachable: open && capability !== "disabled",
                readonly: true,
            };
        }),
    );

    const unsubs: Array<() => void> = [];

    onMount(() => {
        unsubs.push(bedrock.isAuthenticated.subscribe((v) => (authed = v)));
        unsubs.push(bedrock.isRestoringAuth.subscribe((v) => (checkingAuth = v)));
        unsubs.push(bedrock.capability.serverHost.subscribe((v) => (serverHost = v)));
        unsubs.push(bedrock.capability.transferPort.subscribe((v) => (transferPort = v)));
        unsubs.push(bedrock.capability.dnsOverrideHost.subscribe((v) => (dnsOverrideHost = v)));
        unsubs.push(bedrock.capability.status.subscribe((v) => (capability = v)));
        unsubs.push(bedrock.sortedRealms.subscribe((v) => (realms = v)));
        unsubs.push(bedrock.isLoadingRealms.subscribe((v) => (loading = v)));
        unsubs.push(bedrock.favorites.subscribe((v) => (favourites = v)));
        unsubs.push(bedrock.activeRealmId.subscribe((v) => (activeId = v)));
        unsubs.push(bedrock.activeRealmName.subscribe((v) => (activeName = v)));
        unsubs.push(bedrock.interfaces.subscribe((v) => (interfaces = v)));

        void bedrock.loadInterfaces();
        void bedrock.initializeRealmsAccess();
    });

    onDestroy(() => {
        for (const off of unsubs) off();
    });

    async function copy(text: string): Promise<void> {
        await navigator.clipboard?.writeText(text).catch(() => {});
    }

    function connect(id: string): void {
        const realm = realms.find((r) => String(r.id) === id);
        if (realm) void bedrock.connectToRealm(realm);
    }
</script>

<div class="rad-section">
    {#if checkingAuth}
        <div class="rad-card">
            <div class="rad-empty" style="padding: 34px 20px">
                <Loader loading size={72} />
                <span class="rad-empty__note">Checking your Microsoft sign-in.</span>
            </div>
        </div>
    {:else if !authed}
        <div class="rad-card">
            <div class="rad-account rad-account--stack">
                <span class="rad-account__badge" style="background: #107c10">MS</span>
                <span class="rad-account__text">
                    <span class="rad-account__name">Sign in with Microsoft</span>
                    <span class="rad-account__meta">TO LIST THE REALMS YOU CAN JOIN</span>
                </span>
                <button class="rad-btn rad-btn--primary" onclick={() => void bedrock.openLoginModal()}>
                    Sign in
                </button>
            </div>
        </div>
    {:else}
        <div class="rad-section__head" style="font-size: var(--text-rad-lead)">Your Realms</div>

        <ListShell
            state={listState}
            count={plates.length}
            failTitle="This server will not accept a Realm"
            failNote="Bedrock support is turned off here, so a Realm has nowhere to send position. Nothing here will have an effect until the operator turns it on."
            retryLabel="Check again"
            onretry={() => void bedrock.capability.refresh()}
            emptyTitle="No Realms on this account"
            emptyNote="Realms you own or have been invited to appear here. If you have just been invited, accept it in Minecraft first."
        >
            <PlateGrid
                {plates}
                onconnect={connect}
                onstop={() => void bedrock.stopRealms()}
                onfavourite={(id) => void bedrock.toggleFavorite(BigInt(id))}
            />
        </ListShell>

        <div class="rad-card">
            <ListenAddresses
                {interfaces}
                port={LISTEN_PORT}
                bind={effectiveBind}
                singleLabel="Point Minecraft here"
                singleNote="Add this as a server in Minecraft and join it instead."
                listLabel="Point Minecraft at one of these"
                listNote="The proxy answers on all of them. Use the one for the device you play on."
                includeLoopback
                empty="No network address yet. Connect to Wi-Fi and reopen this pane."
            />

            <RelayAddresses addresses={relayAddresses} />

            {#if !mobile}
                <SettingRow
                    label="Listen on"
                    note="Which of this machine's addresses the proxy binds. Loopback unless you play from a console or a phone on this network."
                >
                    {#snippet control()}
                        <select
                            class="rad-select"
                            value={bind}
                            aria-label="Listen on"
                            onchange={(e) => (bind = (e.target as HTMLSelectElement).value)}
                        >
                            {#each choices as choice (choice.id)}
                                <option value={choice.bind}>{choice.label}</option>
                            {/each}
                        </select>
                    {/snippet}
                </SettingRow>
            {/if}

            <SettingRow label="Forwarding to">
                {#snippet control()}
                    <StatusChip severity={activeName ? "ok" : "muted"}>
                        {activeName || "Nothing yet"}
                    </StatusChip>
                {/snippet}
            </SettingRow>
        </div>

        <div class="rad-callout rad-callout--warn">
            <span>
                Join <b>{join}</b> in Minecraft, not the Realm. Joining the Realm from the Friends
                tab skips the proxy and voice stays non-positional.
            </span>
        </div>

        <BedrockLog {bedrock} {mobile} live={activeId !== null} />
    {/if}
</div>

<XboxSignIn {bedrock} />
