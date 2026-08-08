<script lang="ts">
  import { I18n } from "$lib/i18n";
    import Fault from "$radial/components/Fault.svelte";
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
  A failed connect during login is the same event as CONN01 on the error route, so it gets
  the same severed ring rather than the empty one it used to borrow — an empty ring means
  nobody is in range, which is a resting state and not this.
-->
<RadScreen label={I18n.t("Not connected")}>
    <div class="rad-split">
        <div class="rad-visual-pane">
            <div class="rad-visual">
                <Fault icon="unlink" />
                <span class="rad-caption">
                    <span class="rad-label">{I18n.t("Voice path")}</span>
                    <span class="rad-caption__value">{I18n.t("NO ANSWER · CONN01")}</span>
                </span>
            </div>
        </div>
        <div class="rad-content-pane">
            <span class="rad-label rad-rise" style="--d: 50">{I18n.t("Not connected")}</span>
            <h2 class="rad-display rad-rise" style="--d: 120; margin-top: 12px; font-size: 2rem">
                {I18n.t("We couldn't reach")}<br /><b>{server}</b>
            </h2>
            <p class="rad-body rad-rise" style="--d: 210">
                {I18n.t("Make sure your BVC server is running, and that your account has permission to use it.")}
            </p>
            <div class="rad-rise" style="--d: 300; margin-top: 22px; max-width: 370px">
                <button class="rad-btn rad-btn--lg rad-btn--primary" style="width: 100%" onclick={onretry}>
                    {I18n.t("Try again")}
                </button>
                <button class="rad-btn rad-btn--lg" style="width: 100%; margin-top: 10px" onclick={onchangeserver}>
                    {I18n.t("Connect to a different server")}
                </button>
            </div>
            <span class="rad-label rad-rise" style="--d: 360; display: block; margin-top: 26px">
                {I18n.t("Still stuck?")}
            </span>
            <div class="rad-swatchrow rad-rise" style="--d: 390">
                <button class="rad-pill-link" onclick={onwiki}>
                    {I18n.t("Wiki")} <span class="rad-pill-link__ext">&#8599;</span>
                </button>
                <button class="rad-pill-link" onclick={ondiscord}>
                    Discord <span class="rad-pill-link__ext">&#8599;</span>
                </button>
            </div>
        </div>
    </div>

    {#snippet footbar()}
        <span class="rad-label">{I18n.t("A wrong address is the usual cause")}</span>
        <span class="rad-label rad-num">v{appVersion}</span>
    {/snippet}
</RadScreen>
