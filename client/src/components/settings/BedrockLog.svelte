<script lang="ts">
  import { I18n } from "$lib/i18n";
    import { onDestroy, onMount } from "svelte";
    import Icon from "$radial/components/Icon.svelte";
    import StatusChip from "$radial/components/StatusChip.svelte";
    import type { BedrockManager } from "../../js/app/managers/bedrock/BedrockManager";
    import type { BedrockLogEntry } from "../../js/bindings/BedrockLogEntry";

    interface Props {
        bedrock: BedrockManager;
        /** Collapsed to begin with, since the log is the longest thing on the pane. */
        mobile?: boolean;
        live?: boolean;
    }
    let { bedrock, mobile = false, live = false }: Props = $props();

    let lines = $state<readonly BedrockLogEntry[]>([]);
    let open = $state(false);

    let body = $state<HTMLElement | null>(null);

    const unsubs: Array<() => void> = [];

    onMount(() => {
        open = !mobile;
        unsubs.push(bedrock.realmsLogs.subscribe((v) => (lines = v)));
    });

    onDestroy(() => {
        for (const off of unsubs) off();
    });

    // Follows the tail only when already at the tail.
    $effect(() => {
        void lines.length;
        if (!body || !open) return;
        const atTail = body.scrollHeight - body.scrollTop - body.clientHeight < 40;
        if (atTail) body.scrollTop = body.scrollHeight;
    });

    function level(entry: BedrockLogEntry): string {
        const value = entry.level.toLowerCase();
        if (value.startsWith("warn")) return "warn";
        if (value.startsWith("err")) return "err";
        return "info";
    }

    function stamp(entry: BedrockLogEntry): string {
        return new Date(Number(entry.timestamp_ms)).toLocaleTimeString([], {
            hour: "2-digit",
            minute: "2-digit",
            second: "2-digit",
        });
    }

    async function copy(): Promise<void> {
        const text = lines
            .map((entry) => `${stamp(entry)} ${entry.level} ${entry.target} ${entry.message}`)
            .join("\n");
        await navigator.clipboard?.writeText(text).catch(() => {});
    }
</script>

<div class="rad-disclosure" class:is-open={open}>
    <button
        class="rad-disclosure__head"
        aria-expanded={open}
        onclick={() => (open = !open)}
    >
        <Icon name="terminal" /> Connection log
        {#if live}
            <StatusChip severity="ok">{I18n.t("Live")}</StatusChip>
        {:else if lines.length}
            <StatusChip severity="muted">{lines.length}</StatusChip>
        {/if}
        <span class="rad-disclosure__caret"><Icon name="chev" /></span>
    </button>

    <div class="rad-disclosure__body">
        <div class="rad-log" bind:this={body}>
            {#each lines as entry, i (`${entry.timestamp_ms}-${i}`)}
                <div class="rad-log__line">
                    <span class="rad-log__ts">{stamp(entry)}</span>
                    <span class="rad-log__level rad-log__level--{level(entry)}">
                        {level(entry)}
                    </span>
                    <span class="rad-log__msg">{entry.message}</span>
                </div>
            {:else}
                <div class="rad-log__line">
                    <span class="rad-log__msg">{I18n.t("Nothing yet. Connect, and this fills in.")}</span>
                </div>
            {/each}
        </div>

        <div class="rad-log-bar">
            <span class="rad-spacer"></span>
            <button class="rad-btn" onclick={() => void copy()}>
                <Icon name="copy" /> {I18n.t("Copy")}
            </button>
            <button class="rad-btn" onclick={() => bedrock.clearLogs()}>{I18n.t("Clear")}</button>
        </div>
    </div>
</div>
