<script lang="ts">
    import { onDestroy, onMount } from "svelte";
    import Icon from "$radial/components/Icon.svelte";
    import SettingRow from "$radial/components/SettingRow.svelte";
    import StatusChip from "$radial/components/StatusChip.svelte";
    import Toggle from "$radial/components/Toggle.svelte";
    import { AboutManager } from "../../../js/app/managers/settings/AboutManager";
    import { UpdateStatus, type UpdateState } from "../../../js/app/settings/UpdateStatus";
    import type { AppInfo } from "../../../js/bindings/AppInfo";

    interface Props {
        /** Shared with the shell, which badges the nav from the same object. */
        updates: UpdateStatus;
    }
    let { updates }: Props = $props();

    const about = new AboutManager();

    let info = $state<AppInfo | null>(null);
    let telemetry = $state(true);
    let mobile = $state(false);
    let exporting = $state(false);
    let showPlatformId = $state(false);
    let platformId = $state("");
    let refreshing = $state(false);
    let refreshMessage = $state("");
    let update = $state<UpdateState>({ kind: "idle", version: null, checkedAt: null });

    const unsubs: Array<() => void> = [];

    onMount(() => {
        unsubs.push(about.appInfo.subscribe((v) => (info = v)));
        unsubs.push(about.telemetry.subscribe((v) => (telemetry = v)));
        unsubs.push(about.isMobile.subscribe((v) => (mobile = v)));
        unsubs.push(about.isExporting.subscribe((v) => (exporting = v)));
        unsubs.push(about.showPlatformId.subscribe((v) => (showPlatformId = v)));
        unsubs.push(about.platformId.subscribe((v) => (platformId = v)));
        unsubs.push(about.isRefreshingFlags.subscribe((v) => (refreshing = v)));
        unsubs.push(about.refreshFlagsMessage.subscribe((v) => (refreshMessage = v)));
        unsubs.push(updates.state.subscribe((v) => (update = v)));
        void about.initialize();
    });

    onDestroy(() => {
        for (const off of unsubs) off();
    });

    // Each state gets its own sentence. "Update status: unknown" is a row that has
    // stopped being about updates and started being about itself.
    const headline = $derived(
        update.kind === "checking"
            ? "Checking for an update"
            : update.kind === "available"
              ? `Version ${update.version} is ready to install`
              : update.kind === "unavailable"
                ? "Updates are installed from wherever you got the app"
                : update.kind === "failed"
                  ? "Couldn't reach the update server"
                  : "You are up to date",
    );

    const detail = $derived(
        update.kind === "unavailable"
            ? "This build does not update itself — your store or package manager does."
            : update.kind === "failed"
              ? "Your connection or the update server. Nothing is wrong with this build."
              : update.checkedAt
                ? `Last checked ${new Date(update.checkedAt).toLocaleTimeString()}.`
                : "Not checked yet.",
    );

    async function copy(text: string): Promise<void> {
        await navigator.clipboard?.writeText(text).catch(() => {});
    }
</script>

<div class="rad-section">
    <div class="rad-section__note">Proximity voice for Minecraft Bedrock. Source available.</div>

    <div class="rad-card">
        <SettingRow label={headline} note={detail}>
            {#snippet control()}
                {#if update.kind === "checking"}
                    <StatusChip severity="idle">Checking</StatusChip>
                {:else if update.kind === "available"}
                    <button class="rad-btn rad-btn--primary">
                        <Icon name="download" /> Install
                    </button>
                {:else if update.kind !== "unavailable"}
                    <button class="rad-btn" onclick={() => void updates.check()}>Check again</button>
                {/if}
            {/snippet}
        </SettingRow>
    </div>

    <div class="rad-card">
        <div class="rad-card__head">Build</div>
        <dl class="rad-deflist">
            <dt>Version</dt>
            <dd><span>{info?.app_version ?? "—"}</span></dd>
            <dt>Protocol</dt>
            <dd><span>{info?.protocol_version ?? "—"}</span></dd>
            <dt>Release</dt>
            <dd>
                <!-- Three presses reveals the platform identifier. It is support's first
                     question and nobody else's business: a row that is always there
                     invites a player to read a machine id as something they should
                     understand. -->
                <!-- A bare span, not a button. A button's padding pushed the value out of
                     line with every other row in the list, and the three-press reveal is a
                     deliberately undiscoverable gesture rather than an advertised control. -->
                <span
                    class="rad-variant"
                    role="presentation"
                    onclick={() => void about.handleVariantClick()}
                >
                    {info?.build_variant ?? "—"}
                </span>
            </dd>
            <dt>Commit</dt>
            <dd>
                <span>{info?.build_commit ?? "—"}</span>
                <button
                    class="rad-icon-btn"
                    onclick={() => void copy(info?.build_commit ?? "")}
                    aria-label="Copy commit"
                >
                    <Icon name="copy" />
                </button>
            </dd>
            {#if showPlatformId}
                <dt>Platform ID</dt>
                <dd>
                    <span>{platformId}</span>
                    <button
                        class="rad-icon-btn"
                        onclick={() => void about.copyPlatformId()}
                        aria-label="Copy platform ID"
                    >
                        <Icon name="copy" />
                    </button>
                </dd>
            {/if}
        </dl>
    </div>

    <div class="rad-card">
        <div class="rad-card__head">Diagnostics and privacy</div>

        <SettingRow
            label="Send anonymous usage and crash reports"
            note="Anonymous usage statistics and crash reports help us improve the app. No personal data is sent."
        >
            {#snippet control()}
                <Toggle
                    checked={telemetry}
                    label="Send anonymous usage and crash reports"
                    onchange={() => void about.handleTelemetryToggle()}
                />
            {/snippet}
        </SettingRow>

        {#if !mobile}
            <SettingRow
                label="Save the logs to a file"
                note="For a bug report. Written to your Documents folder."
            >
                {#snippet control()}
                    <button
                        class="rad-btn"
                        disabled={exporting}
                        onclick={() => void about.handleExportLogs()}
                    >
                        <Icon name="download" />
                        {exporting ? "Saving…" : "Export logs"}
                    </button>
                {/snippet}
            </SettingRow>
        {/if}

        <SettingRow
            label="Check for new features and entitlements"
            note={refreshMessage ||
                "Some users may have access to features or entitlements. You can manually refresh to check."}
        >
            {#snippet control()}
                <button
                    class="rad-btn"
                    disabled={refreshing}
                    onclick={() => void about.handleRefreshFlags()}
                >
                    <Icon name="refresh" spin={refreshing} />
                    Refresh
                </button>
            {/snippet}
        </SettingRow>
    </div>

    <div class="rad-link-grid">
        {#each about.links as link (link.url)}
            <button class="rad-link-card" onclick={() => void copy(link.url)}>
                {link.title} <Icon name="ext" />
            </button>
        {/each}
        <button
            class="rad-link-card"
            onclick={() => void copy("https://www.bedrockvoicechat.com/wiki/")}
        >
            Wiki <Icon name="ext" />
        </button>
    </div>
</div>
