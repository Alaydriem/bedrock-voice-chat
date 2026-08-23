<script lang="ts">
  import { I18n } from "$lib/i18n";
    import Ring from "$radial/components/Ring.svelte";
    import RadScreen from "../shell/RadScreen.svelte";

    interface Props {
        onguide: () => void;
        oncopyinvite: () => void;
        onwatch: () => void;
        onwiki: () => void;
        ondiscord: () => void;
        onsignin: () => void;
    }
    let { onguide, oncopyinvite, onwatch, onwiki, ondiscord, onsignin }: Props = $props();
</script>

<RadScreen label={I18n.t("No server yet")}>
    <div class="rad-split">
        <div class="rad-visual-pane">
            <div class="rad-visual">
                <Ring mode="empty" class="rad-ring--fill" />
                <span class="rad-caption">
                    <span class="rad-label">{I18n.t("Nothing in range")}</span>
                    <span class="rad-caption__value">{I18n.t("ONE INSTALL AWAY")}</span>
                </span>
            </div>
        </div>
        <div class="rad-content-pane rad-content-pane--top">
            <span class="rad-label rad-rise" style="--d: 50">{I18n.t("No server yet")}</span>
            <h2 class="rad-display rad-rise" style="--d: 120; margin-top: 12px; font-size: 2rem">
                {I18n.t("Set it up once.")}<br /><b>{I18n.t("Then everyone just signs in.")}</b>
            </h2>
            <p class="rad-body rad-rise" style="--d: 200">
                {I18n.t("Voice chat needs two things: a mod on your world, and a BVC server running somewhere you control. Nothing at all on your players' machines.")}
            </p>

            <span class="rad-label rad-rise" style="--d: 240; display: block; margin-top: 26px">
                {I18n.t("What's involved")}
            </span>
            <div class="rad-steps rad-rise" style="--d: 270">
                <div class="rad-step">
                    <span class="rad-step__n">1</span>
                    <span><span class="rad-step__title">{I18n.t("Run the BVC server")}</span>
                        <span class="rad-step__note">{I18n.t("On a machine you control — your PC, a VPS, or a home box")}</span></span>
                </div>
                <div class="rad-step">
                    <span class="rad-step__n">2</span>
                    <span><span class="rad-step__title">{I18n.t("Add the mod to your world")}</span>
                        <span class="rad-step__note">{I18n.t("Wherever your world is hosted, including Aternos and Realms")}</span></span>
                </div>
                <div class="rad-step">
                    <span class="rad-step__n">3</span>
                    <span><span class="rad-step__title">{I18n.t("Share the address — everyone talks")}</span></span>
                </div>
            </div>

            <span class="rad-label rad-rise" style="--d: 320; display: block; margin-top: 28px">
                {I18n.t("Which one are you?")}
            </span>
            <div class="rad-choices rad-rise" style="--d: 350">
                <button class="rad-choice" onclick={onguide}>
                    <span>
                        <span class="rad-choice__title">{I18n.t("I run the server")}</span>
                        <span class="rad-choice__note">
                            {I18n.t("Do both installs yourself. The guide covers the server and the mod, start to finish, in about fifteen minutes.")}
                        </span>
                    </span>
                    <span class="rad-choice__action">{I18n.t("Install guide →")}</span>
                </button>
                <button class="rad-choice" onclick={oncopyinvite}>
                    <span>
                        <span class="rad-choice__title">{I18n.t("A friend runs the server")}</span>
                        <span class="rad-choice__note">
                            {I18n.t("They only have to do it once. Copy a short message with the guide link and send it over.")}
                        </span>
                    </span>
                    <span class="rad-choice__action">{I18n.t("Copy message →")}</span>
                </button>
                <button class="rad-choice" onclick={onwatch}>
                    <span>
                        <span class="rad-choice__title">{I18n.t("I'm just looking")}</span>
                        <span class="rad-choice__note">
                            {I18n.t("Watch the setup walkthrough and see it running before you commit to anything.")}
                        </span>
                    </span>
                    <span class="rad-choice__action">{I18n.t("Watch it →")}</span>
                </button>
            </div>

            <div class="rad-swatchrow" style="margin-top: 22px">
                <button class="rad-pill-link" onclick={onwiki}>
                    {I18n.t("Wiki")} <span class="rad-pill-link__ext">&#8599;</span>
                </button>
                <button class="rad-pill-link" onclick={ondiscord}>
                    Discord <span class="rad-pill-link__ext">&#8599;</span>
                </button>
                <button class="rad-pill-link" onclick={onwatch}>
                    bedrockvoicechat.com <span class="rad-pill-link__ext">&#8599;</span>
                </button>
            </div>
        </div>
    </div>

    {#snippet footbar()}
        <span class="rad-label">{I18n.t("Come back any time from the sign-in screen")}</span>
        <button class="rad-btn rad-btn--lg" onclick={onsignin}>{I18n.t("I have an address — sign in")}</button>
    {/snippet}
</RadScreen>
