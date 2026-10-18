<script lang="ts">
    import { I18n } from "#lib/i18n/index.js";
    import RadConfirm from "./RadConfirm.svelte";

    interface Props {
        open: boolean;
        onaccept: () => void;
        ondismiss: () => void;
    }
    let { open, onaccept, ondismiss }: Props = $props();
</script>

<!--
  Mounted by the root layout, outside every screen's frame, so it brings its own stage.
  Clicks pass through it while the dialog is closed.
-->
<div class="update-offer-stage">
    <RadConfirm
        {open}
        title={I18n.t("Update available")}
        confirmLabel={I18n.t("Update now")}
        cancelLabel={I18n.t("Not now")}
        onconfirm={onaccept}
        oncancel={ondismiss}
    >
        {#snippet body()}
            {I18n.t(
                "A new version of Bedrock Voice Chat is ready to install. The app restarts when it is done.",
            )}
        {/snippet}
    </RadConfirm>
</div>

<style>
    .update-offer-stage {
        position: fixed;
        inset: 0;
        z-index: 100;
        pointer-events: none;
        font-family: var(--font-rad-sans);
        color: var(--color-rad-tx);
    }
</style>
