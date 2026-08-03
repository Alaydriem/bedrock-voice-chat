<script lang="ts">
    import Ring from "$radial/components/Ring.svelte";
    import RadScreen from "../shell/RadScreen.svelte";

    interface Props {
        server: string;
        appVersion: string;
        onretry: () => void;
        onchangeserver: () => void;
        onwiki: () => void;
        ondiscord: () => void;
    }
    let { server, appVersion, onretry, onchangeserver, onwiki, ondiscord }: Props = $props();
</script>

<!--
  The design has no screen for a failed connect, but the flow has one and it carries
  the only recovery actions available at that point. Composed from the kit rather than
  left on the old card.
-->
<RadScreen label="Not connected">
    <div class="rad-split">
        <div class="rad-visual-pane">
            <div class="rad-visual">
                <Ring mode="empty" class="rad-ring--fill" />
                <span class="rad-caption">
                    <span class="rad-label">No answer</span>
                    <span class="rad-caption__value">CONNECT FAILED</span>
                </span>
            </div>
        </div>
        <div class="rad-content-pane">
            <span class="rad-label rad-rise" style="--d: 50">Not connected</span>
            <h2 class="rad-display rad-rise" style="--d: 120; margin-top: 12px; font-size: 2rem">
                We couldn't reach<br /><b>{server}</b>
            </h2>
            <p class="rad-body rad-rise" style="--d: 210">
                Make sure your BVC server is running, and that your account has permission to
                use it.
            </p>
            <div class="rad-rise" style="--d: 300; margin-top: 22px; max-width: 370px">
                <button class="rad-btn rad-btn--lg rad-btn--primary" style="width: 100%" onclick={onretry}>
                    Try again
                </button>
                <button class="rad-btn rad-btn--lg" style="width: 100%; margin-top: 10px" onclick={onchangeserver}>
                    Connect to a different server
                </button>
            </div>
            <span class="rad-label rad-rise" style="--d: 360; display: block; margin-top: 26px">
                Still stuck?
            </span>
            <div class="rad-swatchrow rad-rise" style="--d: 390">
                <button class="rad-pill-link" onclick={onwiki}>
                    Wiki <span class="rad-pill-link__ext">&#8599;</span>
                </button>
                <button class="rad-pill-link" onclick={ondiscord}>
                    Discord <span class="rad-pill-link__ext">&#8599;</span>
                </button>
            </div>
        </div>
    </div>

    {#snippet footbar()}
        <span class="rad-label">A wrong address is the usual cause</span>
        <span class="rad-label rad-num">v{appVersion}</span>
    {/snippet}
</RadScreen>
