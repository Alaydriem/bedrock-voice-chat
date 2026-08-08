<script lang="ts">
  import { I18n } from "$lib/i18n";
    import ProximityRing from "$radial/components/ProximityRing.svelte";
    import RadScreen from "../shell/RadScreen.svelte";
    import type { ResolveVerdict } from "../../js/app/login/AddressResolver";

    interface Props {
        address: string;
        verdict: ResolveVerdict;
        /**
         * The address is a `code@<host>` entry, so the button leads to the code screen
         * rather than out to Microsoft. Nothing on that path involves a Microsoft
         * account, so the button must not promise one.
         */
        isCode?: boolean;
        appVersion: string;
        /**
         * What to call the way off this screen. Absent on a cold launch, which has nowhere to
         * go back to — an exit that leads back here is worse than none.
         */
        backLabel?: string;
        oninput: (value: string) => void;
        onconnect: () => void;
        onprivacy: () => void;
        onrevisit: () => void;
        onback?: () => void;
    }
    let {
        address,
        verdict,
        isCode = false,
        appVersion,
        backLabel,
        oninput,
        onconnect,
        onprivacy,
        onrevisit,
        onback,
    }: Props = $props();

    // The ring is a readout: quiet while the field is being typed into, alive once a name
    // resolves. `verdict.ring` carries that state; `ProximityRing` fills a live one.

    let resolveClass = $derived(
        verdict.state === "ok"
            ? "rad-resolve rad-resolve--ok"
            : verdict.state === "bad"
              ? "rad-resolve rad-resolve--bad"
              : "rad-resolve",
    );

</script>

<RadScreen label={I18n.t("Connect")}>
    <div class="rad-split">
        <div class="rad-visual-pane">
            <div class="rad-visual">
                <ProximityRing mode={verdict.ring} class="rad-ring--fill" />
                <span class="rad-caption">
                    <span class="rad-label">{I18n.t("Acquiring")}</span>
                    <span class="rad-caption__value">{verdict.caption}</span>
                </span>
            </div>
        </div>
        <div class="rad-content-pane">
            <span class="rad-label rad-rise" style="--d: 50">{I18n.t("Connect")}</span>
            <h2
                class="rad-display rad-rise"
                style="--d: 120; margin-top: 12px; font-size: 2.1111rem"
            >
                {I18n.t("Which server are you joining?")}
            </h2>
            <div class="rad-rise" style="--d: 210; margin-top: 20px; max-width: 370px">
                <span class="rad-label">{I18n.t("Server address")}</span>
                <div class="rad-field">
                    <span class="rad-field__prefix">{I18n.t("ADDR")}</span>
                    <input
                        type="text"
                        value={address}
                        spellcheck="false"
                        autocapitalize="none"
                        autocorrect="off"
                        autocomplete="url"
                        aria-label={I18n.t("Server address")}
                        oninput={(e) => oninput((e.currentTarget as HTMLInputElement).value)}
                    />
                </div>
                <div class={resolveClass}>{verdict.line}</div>
                <!--
                  Never disabled by the verdict. The probe is a readout; a slow or
                  blocked measurement must not be the reason someone cannot sign in.
                -->
                {#if isCode}
                    <button class="rad-btn rad-btn--lg rad-btn--primary" onclick={onconnect}>
                        {I18n.t("Continue with a code")}
                    </button>
                {:else}
                    <button class="rad-ms-signin" onclick={onconnect}>
                        <span class="rad-ms-mark"><i></i><i></i><i></i><i></i></span>{I18n.t("Sign in with Microsoft")}
                    </button>
                {/if}
            </div>
        </div>
    </div>

    {#snippet footbar()}
        <span class="rad-footbar__actions">
            {#if onback && backLabel}
                <button class="rad-btn rad-btn--quiet" onclick={onback}>{backLabel}</button>
            {/if}
            <button class="rad-btn rad-btn--quiet" onclick={onprivacy}>{I18n.t("Privacy notice")}</button>
            <button class="rad-btn rad-btn--quiet" onclick={onrevisit}>{I18n.t("What is this?")}</button>
        </span>
        <span class="rad-label rad-num">v{appVersion}</span>
    {/snippet}
</RadScreen>
