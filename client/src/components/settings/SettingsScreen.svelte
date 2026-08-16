<script lang="ts">
  import { I18n } from "$lib/i18n";
    import { onDestroy, onMount } from "svelte";
    import Icon from "$radial/components/Icon.svelte";
    import ServerGlyph from "$radial/components/ServerGlyph.svelte";
    import { SettingsCatalogue } from "../../js/app/settings/SettingsCatalogue";
    import { BedrockCapabilityManager } from "../../js/app/managers/bedrock/BedrockCapabilityManager";
    import { BedrockManager } from "../../js/app/managers/bedrock/BedrockManager";
    import { UpdateStatus } from "../../js/app/settings/UpdateStatus";
    import PlatformDetector from "../../js/app/utils/PlatformDetector";
    import SettingsNav from "./SettingsNav.svelte";
    import AboutPane from "./panes/AboutPane.svelte";
    import AccountPane from "./panes/AccountPane.svelte";
    import AudioPane from "./panes/AudioPane.svelte";
    import KeybindsPane from "./panes/KeybindsPane.svelte";
    import LibraryPane from "./panes/LibraryPane.svelte";
    import PlayersPane from "./panes/PlayersPane.svelte";
    import ConnectPane from "./panes/ConnectPane.svelte";
    import RecordingsPane from "./panes/RecordingsPane.svelte";
    import WebSocketPane from "./panes/WebSocketPane.svelte";

    interface Props {
        pane: string;
        /**
         * Which of the phone's two screens is showing. Ignored at desktop width, where the
         * section nav and the pane stand side by side as one screen.
         */
        level: "list" | "detail";
        /**
         * Reached without a dashboard behind it — the error screen's "change audio
         * devices" lands here. Nothing is connected, and the screen says so.
         */
        standalone?: boolean;
        /** Servers for the rail. Empty on the standalone route, which has no session. */
        servers?: readonly { host: string }[];
        /**
         * The session's update status, from the shell that polls it. Omitted by the
         * standalone route, which has no shell behind it and checks on demand.
         */
        updates?: UpdateStatus;
        /**
         * Routing is the route's business. The shell says which pane was asked for and
         * that it wants out; where those go is decided by whoever mounted it, which is
         * also what keeps this component testable without a router.
         */
        onnavigate: (pane: string) => void;
        onclose: () => void;
        /** The back button in the bar. One screen up; the route decides what that is. */
        onback: () => void;
        /** Signing out is the session's business, not a pane's. */
        onsignout?: () => void;
    }
    let {
        pane,
        level,
        standalone = false,
        servers = [],
        updates = new UpdateStatus(),
        onnavigate,
        onclose,
        onback,
        onsignout = () => {},
    }: Props = $props();

    /**
     * The mobile build. Not the same as a narrow window, which is a container query.
     *
     * Read synchronously. `plugin-os` returns a value injected at startup, so awaiting it
     * only guarantees a first frame rendered as desktop — the wrong catalogue and the
     * wrong layout — before the right one replaces it.
     */
    const mobile = new PlatformDetector().mobile();

    /** One manager across both Bedrock panes; built on first use. */
    let bedrock: BedrockManager | null = null;
    function bedrockManager(): BedrockManager {
        if (!bedrock) {
            bedrock = new BedrockManager(new BedrockCapabilityManager());
            // Loads both managers from the store and restores the Microsoft session.
            // Guards itself against running twice.
            void bedrock.initialize();
        }
        return bedrock;
    }
    let updateBadge = $state(false);

    const groups = $derived(SettingsCatalogue.groups(mobile));
    const current = $derived(SettingsCatalogue.find(pane, mobile) ?? SettingsCatalogue.all[0]);
    const badged = $derived(updateBadge ? "about" : null);

    let unbadge: (() => void) | null = null;

    onMount(() => {
        unbadge = updates.badge.subscribe((v) => (updateBadge = v));
    });

    onDestroy(() => {
        unbadge?.();
        bedrock?.destroy();
    });

    // A pane change can happen without a remount, so the body is scrolled back itself.
    let body = $state<HTMLElement | null>(null);
    $effect(() => {
        void pane;
        if (body) body.scrollTop = 0;
    });

</script>

<div class="rad-shell rad-settings" class:is-list={level === "list"}>
    <div class="rad-rail">
        <div class="rad-rail__list">
            {#each servers as server (server.host)}
                <button class="rad-rail-item" aria-label={server.host}>
                    <ServerGlyph name={server.host} size={36} />
                </button>
            {/each}
        </div>
        <span class="rad-rail__spacer"></span>
        <button class="rad-rail-btn is-on" onclick={onclose} aria-label={I18n.t("Close settings")}>
            <Icon name="gear" />
        </button>
    </div>

    <div class="rad-panel">
        <div class="rad-panel__head">{I18n.t("Settings")}</div>
        <div class="rad-panel__body">
            <SettingsNav {groups} current={current.id} {badged} onpick={onnavigate} />
        </div>
    </div>

    <div class="rad-stage">
        <div class="rad-dash-top">
            <span class="rad-dash-top__who">
                <span class="rad-dash-top__server">{current.title}</span>
            </span>
            <span class="rad-dash-top__state">
                <button class="rad-header-btn" onclick={onclose} aria-label={I18n.t("Close settings")}>
                    <Icon name="close" />
                </button>
            </span>
        </div>

        <div class="rad-backbar">
            <button class="rad-backbar__btn" onclick={onback} aria-label={I18n.t("Back")}>
                <Icon name={level === "list" ? "close" : "back"} />
            </button>
            <span class="rad-backbar__title">
                {level === "list" ? "Settings" : current.title}
            </span>
        </div>

        <div class="rad-mobile-list">
            <SettingsNav {groups} current={current.id} {badged} layout="list" onpick={onnavigate} />
        </div>

        <div class="rad-settings-body" bind:this={body}>
            <div class="rad-settings-measure" class:is-wide={current.wide}>
                {#if standalone}
                    <div class="rad-callout" style="margin-bottom: 14px">
                        <span>
                            {I18n.t("You are in settings on its own.")} <b>{I18n.t("Nothing is connected")}</b> — go back to
                            the dashboard when you are done here.
                        </span>
                    </div>
                {/if}

                <!-- Keyed so a pane's subscriptions and polls are torn down on leaving. -->
                {#key current.id}
                    {#if current.id === "account"}
                        <AccountPane {onsignout} />
                    {:else if current.id === "audio"}
                        <AudioPane {mobile} />
                    {:else if current.id === "players"}
                        <PlayersPane />
                    {:else if current.id === "recordings"}
                        <RecordingsPane />
                    {:else if current.id === "library"}
                        <LibraryPane />
                    {:else if current.id === "keybinds"}
                        <KeybindsPane />
                    {:else if current.id === "ws"}
                        <WebSocketPane />
                    {:else if current.id === "connect"}
                        <ConnectPane bedrock={bedrockManager()} {mobile} />
                    {:else if current.id === "about"}
                        <AboutPane {updates} />
                    {/if}
                {/key}
            </div>
        </div>
    </div>
</div>
