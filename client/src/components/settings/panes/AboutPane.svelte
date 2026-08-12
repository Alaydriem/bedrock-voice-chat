<script lang="ts">
    import { onDestroy, onMount } from "svelte";
    import Icon from "$radial/components/Icon.svelte";
    import IdField from "$radial/components/IdField.svelte";
    import SettingRow from "$radial/components/SettingRow.svelte";
    import StatusChip from "$radial/components/StatusChip.svelte";
    import Toggle from "$radial/components/Toggle.svelte";
    import { AboutManager } from "../../../js/app/managers/settings/AboutManager";
    import BVCApp from "../../../js/app/BVCApp";
    import type LocaleManager from "../../../js/app/managers/settings/LocaleManager";
    import { UpdateStatus, type UpdateState } from "../../../js/app/settings/UpdateStatus";
    import type { AppInfo } from "../../../js/bindings/AppInfo";
    import { I18n } from "$lib/i18n";

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
    let platformId = $state("");
    let refreshingPlatformId = $state(false);
    let platformIdError = $state("");
    let refreshing = $state(false);
    let refreshMessage = $state("");
    let update = $state<UpdateState>({ kind: "idle", version: null, checkedAt: null });
    let locales = $state<string[]>([]);
    let activeLocale = $state("auto");
    let locale = $state<LocaleManager | null>(null);

    const unsubs: Array<() => void> = [];
    let destroyed = false;

    onMount(() => {
        unsubs.push(about.appInfo.subscribe((v) => (info = v)));
        unsubs.push(about.telemetry.subscribe((v) => (telemetry = v)));
        unsubs.push(about.isMobile.subscribe((v) => (mobile = v)));
        unsubs.push(about.isExporting.subscribe((v) => (exporting = v)));
        unsubs.push(about.platformId.subscribe((v) => (platformId = v)));
        unsubs.push(about.isRefreshingPlatformId.subscribe((v) => (refreshingPlatformId = v)));
        unsubs.push(about.platformIdError.subscribe((v) => (platformIdError = v)));
        unsubs.push(about.isRefreshingFlags.subscribe((v) => (refreshing = v)));
        unsubs.push(about.refreshFlagsMessage.subscribe((v) => (refreshMessage = v)));
        unsubs.push(updates.state.subscribe((v) => (update = v)));
        void about.initialize();

        // Already resolved by the time any screen renders; awaited rather than injected so
        // this pane works on the standalone settings route, which has no shell above it.
        //
        // Subscribing after a destroy would outlive the pane: the manager is owned by the
        // app, so nothing else would ever drop the subscription.
        void BVCApp.localeManager().then((manager) => {
            if (destroyed) return;
            locale = manager;
            unsubs.push(manager.locales.subscribe((v) => (locales = v)));
            unsubs.push(manager.active.subscribe((v) => (activeLocale = v)));
        });
    });

    onDestroy(() => {
        destroyed = true;
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
    <div class="rad-section__note">{I18n.t("Proximity voice for Minecraft Bedrock. Source available.")}</div>

    <div class="rad-card">
        <SettingRow label={headline} note={detail}>
            {#snippet control()}
                {#if update.kind === "checking"}
                    <StatusChip severity="idle">{I18n.t("Checking")}</StatusChip>
                {:else if update.kind === "available"}
                    <button class="rad-btn rad-btn--primary">
                        <Icon name="download" /> {I18n.t("Install")}
                    </button>
                {:else if update.kind !== "unavailable"}
                    <button class="rad-btn" onclick={() => void updates.check()}>{I18n.t("Check again")}</button>
                {/if}
            {/snippet}
        </SettingRow>
    </div>

    <div class="rad-card">
        <div class="rad-card__head">{I18n.t("Build")}</div>
        <dl class="rad-deflist">
            <dt>{I18n.t("Version")}</dt>
            <dd><span>{info?.app_version ?? "—"}</span></dd>
            <dt>{I18n.t("Protocol")}</dt>
            <dd><span>{info?.protocol_version ?? "—"}</span></dd>
            <dt>{I18n.t("Release")}</dt>
            <dd><span>{info?.build_variant ?? "—"}</span></dd>
            <dt>{I18n.t("Commit")}</dt>
            <dd>
                <span>{info?.build_commit ?? "—"}</span>
                <button
                    class="rad-icon-btn"
                    onclick={() => void copy(info?.build_commit ?? "")}
                    aria-label={I18n.t("Copy commit")}
                >
                    <Icon name="copy" />
                </button>
            </dd>
        </dl>
    </div>

    <div class="rad-card">
        <div class="rad-card__head">{I18n.t("Language")}</div>

        <SettingRow
            label={I18n.t("Display language")}
        >
            {#snippet control()}
                <select
                    class="rad-select"
                    value={activeLocale}
                    disabled={locale === null}
                    onchange={(event) => void locale?.choose(event.currentTarget.value)}
                >
                    <option value="auto">{I18n.t("Match my system")}</option>
                    {#each locales as available (available)}
                        <option value={available}>{available}</option>
                    {/each}
                </select>
            {/snippet}
        </SettingRow>
    </div>

    <div class="rad-card">
        <div class="rad-card__head">{I18n.t("Diagnostics and privacy")}</div>

        <SettingRow
            label={I18n.t("Send anonymous usage and crash reports")}
            note={I18n.t("Anonymous usage statistics and crash reports help us improve the app. No personal data is sent.")}
        >
            {#snippet control()}
                <Toggle
                    checked={telemetry}
                    label={I18n.t("Send anonymous usage and crash reports")}
                    onchange={() => void about.handleTelemetryToggle()}
                />
            {/snippet}
        </SettingRow>

        <SettingRow
            label={I18n.t("Platform ID")}
            note={platformIdError ||
                I18n.t("Uniquely identifies your build for experimental features + analytics.")}
            stack
        >
            <IdField value={platformId || "—"} copyLabel={I18n.t("Copy platform ID")}>
                {#snippet actions()}
                    <button
                        class="rad-btn"
                        disabled={refreshingPlatformId}
                        onclick={() => void about.refreshPlatformId()}
                    >
                        <Icon name="refresh" spin={refreshingPlatformId} />
                        {I18n.t("Refresh")}
                    </button>
                {/snippet}
            </IdField>
        </SettingRow>

        {#if !mobile}
            <SettingRow
                label={I18n.t("Save the logs to a file")}
                note={I18n.t("For a bug report. Written to your Documents folder.")}
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
            label={I18n.t("Check for new features and entitlements")}
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
                    {I18n.t("Refresh")}
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
            {I18n.t("Wiki")} <Icon name="ext" />
        </button>
    </div>
</div>
