<script lang="ts">
    import { I18n } from "#lib/i18n/index.js";
    import Icon from "$radial/components/Icon.svelte";

    interface Props {
        name: string;
        /** This client is the last member, so leaving closes the group. */
        closes: boolean;
        onconfirm: () => void;
        oncancel: () => void;
    }
    let { name, closes, onconfirm, oncancel }: Props = $props();

    function onkeydown(event: KeyboardEvent): void {
        if (event.key === "Escape") oncancel();
    }
</script>

<svelte:window {onkeydown} />

<div class="rad-scrim rad-scrim--modal is-on" aria-hidden="true"></div>
<div class="rad-modal rad-modal--dock is-open" role="dialog" aria-modal="true" aria-labelledby="leave-private-title">
    <div class="rad-dialog-head">
        <span class="rad-seal"><Icon name="lock" /></span>
        <h5 class="rad-modal__title" id="leave-private-title">
            {#if closes}
                {I18n.tf("Close {name}?", { name })}
            {:else}
                {I18n.tf("Leave {name}?", { name })}
            {/if}
        </h5>
    </div>

    {#if closes}
        <p>{I18n.t("You are the last member. Leaving closes the group.")}</p>
    {:else}
        <p>{I18n.t("To come back, a member must admit you.")}</p>
    {/if}

    <div class="rad-modal__actions">
        <button class="rad-btn" onclick={oncancel}>{I18n.t("Cancel")}</button>
        {#if closes}
            <button class="rad-btn rad-btn--danger" onclick={onconfirm}>{I18n.t("Leave and close")}</button>
        {:else}
            <button class="rad-btn rad-btn--primary" onclick={onconfirm}>{I18n.t("Leave")}</button>
        {/if}
    </div>
</div>
