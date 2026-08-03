<script lang="ts">
    import type { Snippet } from "svelte";
    import Mark from "$radial/components/Mark.svelte";

    interface Props {
        /** Right-hand label in the top bar, when the screen needs no richer chrome. */
        label?: string;
        /** Replaces `label` when the top bar carries a control, such as step dots. */
        topbar?: Snippet;
        footbar?: Snippet;
        children: Snippet;
    }
    let { label, topbar, footbar, children }: Props = $props();
</script>

<!--
  `is-on` is unconditional: only one screen is rendered at a time, so visibility is
  the router's job rather than a class toggle. The class stays because the kit's
  layout is written against it.
-->
<section class="rad-screen is-on">
    <div class="rad-topbar">
        <!--
          The component owns its canvas. A bare `data-rad-mark` canvas would need
          Mount.scan to find it, and that scan also claims every other radial canvas
          on the page — including ones Svelte components already own, which left two
          bindings painting the same meter.
        -->
        <span class="rad-brand">
            <Mark />
            <span class="rad-wordmark">Bedrock Voice Chat</span>
        </span>
        {#if topbar}{@render topbar()}{:else if label}
            <span class="rad-label">{label}</span>
        {/if}
    </div>

    {@render children()}

    {#if footbar}
        <div class="rad-footbar">{@render footbar()}</div>
    {/if}
</section>
