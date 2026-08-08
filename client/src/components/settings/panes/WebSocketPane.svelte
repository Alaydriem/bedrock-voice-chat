<script lang="ts">
  import { I18n } from "$lib/i18n";
    import { invoke } from "@tauri-apps/api/core";
    import { onDestroy, onMount } from "svelte";
    import Icon from "$radial/components/Icon.svelte";
    import SettingRow from "$radial/components/SettingRow.svelte";
    import StatusChip from "$radial/components/StatusChip.svelte";
    import Toggle from "$radial/components/Toggle.svelte";
    import { WebSocketSettingsManager } from "../../../js/app/managers/settings/WebSocketSettingsManager";
    import type { NetworkInterface } from "../../../js/bindings/NetworkInterface";
    import type { WebSocketClientInfo } from "../../../js/bindings/WebSocketClientInfo";
    import { ListenAddress } from "../../../js/app/settings/ListenAddress";

    const ws = new WebSocketSettingsManager();

    let running = $state(false);
    let localhostOnly = $state(true);
    let port = $state("8444");
    let authKey = $state("");
    let clients = $state<readonly WebSocketClientInfo[]>([]);
    let mobile = $state(false);
    let startError = $state("");
    let interfaces = $state<readonly NetworkInterface[]>([]);

    // Bound to every interface, `0.0.0.0` is not connectable. Which address to hand out is
    // not decidable here, so every plausible one is listed.
    const reachable = $derived(ListenAddress.candidates(interfaces, Number(port) || 0));
    let now = $state(Math.floor(Date.now() / 1000));

    const address = $derived(`ws://${localhostOnly ? "127.0.0.1" : "0.0.0.0"}:${port}`);

    const unsubs: Array<() => void> = [];
    let poll: ReturnType<typeof setInterval> | null = null;

    async function refreshClients(): Promise<void> {
        if (!running) {
            clients = [];
            return;
        }
        try {
            clients = await invoke<WebSocketClientInfo[]>("websocket_clients");
        } catch {
            clients = [];
        }
    }

    onMount(() => {
        unsubs.push(ws.isRunning.subscribe((v) => (running = v)));
        unsubs.push(ws.localhostOnly.subscribe((v) => (localhostOnly = v)));
        unsubs.push(ws.websocketPort.subscribe((v) => (port = v)));
        unsubs.push(ws.authKey.subscribe((v) => (authKey = v)));
        unsubs.push(ws.isMobile.subscribe((v) => (mobile = v)));
        unsubs.push(ws.startError.subscribe((v) => (startError = v)));
        void ws.initialize();
        void invoke<NetworkInterface[]>("bedrock_list_interfaces")
            .then((v) => (interfaces = v))
            .catch(() => {});

        poll = setInterval(() => {
            now = Math.floor(Date.now() / 1000);
            void refreshClients();
        }, 2_000);
        void refreshClients();
    });

    onDestroy(() => {
        for (const off of unsubs) off();
        if (poll) clearInterval(poll);
    });

    function clock(seconds: number): string {
        const safe = Math.max(0, seconds);
        const parts = [Math.floor(safe / 3600), Math.floor((safe % 3600) / 60), safe % 60];
        return parts.map((n) => String(n).padStart(2, "0")).join(":");
    }

    async function copy(text: string): Promise<void> {
        await navigator.clipboard?.writeText(text).catch(() => {});
    }
</script>

<div class="rad-section">
    <div class="rad-section__note">
        {I18n.t("Enabling the Websocket Server lets you connect to BVC from other devices, such as a Stream Deck. Recommended for content creators and streamers.")}
    </div>

    <div class="rad-card">
        <SettingRow
            label={I18n.t("Enable the server")}
            note={I18n.t("Off by default. Nothing can drive the client until this is on.")}
        >
            {#snippet control()}
                <StatusChip severity={running ? "ok" : "muted"}>
                    {running ? "Listening" : "Stopped"}
                </StatusChip>
                <Toggle
                    checked={running}
                    label={I18n.t("Enable the WebSocket server")}
                    onchange={() => void ws.handleToggleServer()}
                />
            {/snippet}
        </SettingRow>

        {#if running}
            <SettingRow
                label={I18n.t("Listen on this device only")}
                note={mobile
                    ? "On mobile, this is always off."
                    : "Turn this off only to drive BVC from another device on your network. It exposes the port to everything that can reach you."}
            >
                {#snippet control()}
                    <Toggle
                        checked={mobile ? false : localhostOnly}
                        disabled={mobile}
                        label={I18n.t("Listen on this device only")}
                        onchange={() => void ws.handleLocalhostToggle()}
                    />
                {/snippet}
            </SettingRow>

            <SettingRow
                label={I18n.t("Port")}
                note={I18n.t("Changing it restarts the server and drops anything connected.")}
            >
                {#snippet control()}
                    <span class="rad-input" style="width: 104px">
                        <input
                            type="text"
                            inputmode="numeric"
                            value={port}
                            aria-label={I18n.t("Port")}
                            onchange={(e) =>
                                void ws.handlePortChange((e.target as HTMLInputElement).value)}
                        />
                    </span>
                {/snippet}
            </SettingRow>

            {#if localhostOnly}
                <SettingRow label={I18n.t("Address")} note={I18n.t("Point your plugin here.")}>
                    {#snippet control()}
                        <span class="rad-input" style="width: 230px">
                            <input type="text" value={address} readonly aria-label={I18n.t("Address")} />
                        </span>
                        <button
                            class="rad-icon-btn"
                            onclick={() => void copy(address)}
                            aria-label={I18n.t("Copy address")}
                        >
                            <Icon name="copy" />
                        </button>
                    {/snippet}
                </SettingRow>
            {:else}
                <SettingRow
                    label={I18n.t("Addresses")}
                    note={I18n.t("This device answers on all of them. Use whichever your other device can reach.")}
                    stack
                >
                    <div class="rad-addresses">
                        {#each reachable as candidate (candidate.address)}
                            <div class="rad-address">
                                <span class="rad-address__value">ws://{candidate.address}</span>
                                <span class="rad-address__label">{candidate.label}</span>
                                <button
                                    class="rad-icon-btn"
                                    onclick={() => void copy(`ws://${candidate.address}`)}
                                    aria-label={I18n.tf("Copy ws://{address}", { address: candidate.address })}
                                >
                                    <Icon name="copy" />
                                </button>
                            </div>
                        {:else}
                            <span class="rad-address__label">
                                {I18n.t("No network address yet. Connect to Wi-Fi and reopen this pane.")}
                            </span>
                        {/each}
                    </div>
                </SettingRow>
            {/if}

            <SettingRow
                label={I18n.t("Access token")}
                note={I18n.t("Required on connect. Regenerating disconnects anything using the old one.")}
            >
                {#snippet control()}
                    <span class="rad-input" style="width: 230px">
                        <input
                            type="password"
                            value={authKey}
                            readonly
                            aria-label={I18n.t("Access token")}
                        />
                    </span>
                    <button
                        class="rad-icon-btn"
                        onclick={() => void copy(authKey)}
                        aria-label={I18n.t("Copy token")}
                    >
                        <Icon name="copy" />
                    </button>
                    <button class="rad-btn" onclick={() => void ws.handleGenerateKey()}>
                        {I18n.t("Regenerate")}
                    </button>
                {/snippet}
            </SettingRow>
        {/if}
    </div>

    {#if startError}
        <div class="rad-callout rad-callout--bad"><span>{startError}</span></div>
    {/if}

    {#if running}
        <div class="rad-section" style="margin-top: 26px">
            <div class="rad-section__head" style="font-size: var(--text-rad-lead)">
                {I18n.t("Connected clients")}
            </div>
            <div class="rad-card">
                <div class="rad-table-wrap">
                    <table class="rad-table">
                        <thead>
                            <tr>
                                <th>{I18n.t("Client")}</th>
                                <th>{I18n.t("Endpoint")}</th>
                                <th class="rad-num">{I18n.t("Connected")}</th>
                                <th class="rad-num">{I18n.t("Commands")}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {#each clients as client (client.id)}
                                <tr>
                                    <td><span class="rad-table__name">{client.name}</span></td>
                                    <td>{client.route}</td>
                                    <td class="rad-num">
                                        {clock(now - Number(client.connected_at))}
                                    </td>
                                    <td class="rad-num">{client.commands}</td>
                                </tr>
                            {:else}
                                <tr>
                                    <td colspan="4" class="rad-table__nomatch">
                                        {I18n.t("Nothing is connected yet.")}
                                    </td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                </div>
            </div>

            <div class="rad-link-grid">
                <button
                    class="rad-link-card"
                    onclick={() =>
                        void copy("https://www.bedrockvoicechat.com/wiki/creator/websocket-api/")}
                >
                    {I18n.t("WebSocket API")} <Icon name="ext" />
                </button>
                <button
                    class="rad-link-card"
                    onclick={() =>
                        void copy("https://www.bedrockvoicechat.com/wiki/creator/stream-deck/")}
                >
                    {I18n.t("Stream Deck plugin")} <Icon name="ext" />
                </button>
            </div>
        </div>
    {/if}
</div>
