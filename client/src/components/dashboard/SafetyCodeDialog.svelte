<script lang="ts">
    import { I18n } from "#lib/i18n/index.js";
    import Icon from "$radial/components/Icon.svelte";

    interface Props {
        open: boolean;
        /** Groups of digits separated by spaces, from the group's current epoch. */
        code: string | null;
        groupName: string;
        memberCount: number;
        onclose: () => void;
    }
    let { open, code, groupName, memberCount, onclose }: Props = $props();

    const cells = $derived(code ? code.split(" ").filter(Boolean) : []);
    let copied = $state(false);

    async function copy(): Promise<void> {
        if (!code) return;
        try {
            await navigator.clipboard.writeText(code);
            copied = true;
            setTimeout(() => (copied = false), 1600);
        } catch {
            copied = false;
        }
    }

    function onkeydown(event: KeyboardEvent): void {
        if (open && event.key === "Escape") onclose();
    }
</script>

<svelte:window {onkeydown} />

<!-- Not in the DOM until asked for: the code is on demand, never always on screen. -->
{#if open}
    <div class="rad-scrim rad-scrim--modal is-on" onclick={onclose} aria-hidden="true"></div>
    <div class="rad-modal rad-modal--dock is-open" role="dialog" aria-modal="true" aria-label={I18n.t("Safety code")}>
        <div class="rad-dialog-head">
            <span class="rad-seal"><Icon name="shield" /></span>
            <span>
                <h5 class="rad-modal__title">{I18n.t("Safety code")}</h5>
                <span class="rad-dialog-sub">{groupName} · {memberCount}</span>
            </span>
        </div>
        <p>{I18n.t("Read it out to the others. If yours is different, leave the group.")}</p>
        <div class="rad-code__grid" aria-label={I18n.t("Safety code")}>
            {#each cells as cell, index (index)}
                <span>{cell}</span>
            {/each}
        </div>
        <div class="rad-modal__actions">
            <button class="rad-btn" onclick={() => void copy()}>
                <Icon name={copied ? "check" : "copy"} />
                {I18n.t("Copy")}
            </button>
            <button class="rad-btn rad-btn--primary" onclick={onclose}>{I18n.t("Done")}</button>
        </div>
    </div>
{/if}
