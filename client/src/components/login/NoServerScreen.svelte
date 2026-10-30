<script lang="ts">
    import { I18n } from "#lib/i18n/index.js";
    import RadScreen from "../shell/RadScreen.svelte";

    interface Props {
        onnodecraft: () => void;
        onguide: () => void;
        oncopyinvite: () => void;
        onsite: () => void;
        onwiki: () => void;
        ondiscord: () => void;
        onsignin: () => void;
        onintro: () => void;
    }
    let {
        onnodecraft,
        onguide,
        oncopyinvite,
        onsite,
        onwiki,
        ondiscord,
        onsignin,
        onintro,
    }: Props = $props();
</script>

<RadScreen label={I18n.t("No server yet")}>
    <div class="rad-split">
        <div class="rad-visual-pane">
            <div class="rad-visual">
                <div class="rad-matrix">
                    <div class="rad-matrix__group">
                        <div class="rad-matrix__head">{I18n.t("Hosted on Nodecraft")}</div>
                        <div class="rad-matrix__row">
                            <span class="rad-matrix__blocks"
                                ><i style="background:#8239d8"></i><i style="background:#6a50e9"
                                ></i><i style="background:#466cf3"></i></span
                            >
                            <span>{I18n.t("Your world and BVC on one server")}</span><span
                                class="rad-matrix__tag rad-matrix__tag--yes"
                                >{I18n.t("Hosted")}</span
                            >
                        </div>
                        <div class="rad-matrix__row">
                            <span class="rad-matrix__blocks"
                                ><i style="background:#28bae1"></i><i style="background:#21d8d8"
                                ></i><i style="background:#26ddcd"></i></span
                            >
                            <span>{I18n.t("Bedrock, Fabric or PaperMC")}</span><span
                                class="rad-matrix__tag rad-matrix__tag--yes"
                                >{I18n.t("Supported")}</span
                            >
                        </div>
                        <div class="rad-matrix__row">
                            <span class="rad-matrix__blocks"
                                ><i style="background:#34d8a0"></i><i style="background:#3bd869"
                                ></i><i style="background:#6fd846"></i></span
                            >
                            <span>{I18n.t("Install, Configure, and Launch")}</span><span
                                class="rad-matrix__tag rad-matrix__tag--yes"
                                >{I18n.t("Guided")}</span
                            >
                        </div>
                    </div>
                    <p class="rad-matrix__note">
                        <span
                            ><b
                                >{I18n.t(
                                    "Get 30% off your first month with this exclusive BVC deal.",
                                )}</b
                            ></span
                        >
                    </p>
                    <button
                        class="rad-btn rad-btn--primary rad-btn--lg"
                        style="align-self: flex-start"
                        onclick={onnodecraft}>{I18n.t("Set up on Nodecraft →")}</button
                    >
                </div>
                <span class="rad-caption">
                    <span class="rad-label">Nodecraft</span>
                    <span class="rad-caption__value">{I18n.t("30% OFF · FIRST MONTH")}</span>
                </span>
            </div>
        </div>
        <div class="rad-content-pane rad-content-pane--top">
            <span class="rad-label rad-rise" style="--d: 50">{I18n.t("No server yet")}</span>
            <h2 class="rad-display rad-rise" style="--d: 120; margin-top: 12px; font-size: 2rem">
                {I18n.t("Create your own Minecraft Server")}<br /><b>{I18n.t("on Nodecraft")}</b>
            </h2>
            <p class="rad-body rad-rise" style="--d: 200">
                {I18n.t(
                    "Nodecraft is a Minecraft hosting service that makes it easy to run your own server with Bedrock Voice Chat. Simply install the custom launcher in Nodecraft, configure your world, and launch. That's it.",
                )}
            </p>

            <span class="rad-label rad-rise" style="--d: 240; display: block; margin-top: 26px">
                {I18n.t("What's involved")}
            </span>
            <div class="rad-steps rad-rise" style="--d: 270">
                <div class="rad-step">
                    <span class="rad-step__n">1</span>
                    <span
                        ><span class="rad-step__title">{I18n.t("Add the custom launcher to your Ndoecraft panel")}</span>
                        <span class="rad-step__note"
                            >{I18n.t("One easy download")}</span
                        ></span
                    >
                </div>
                <div class="rad-step">
                    <span class="rad-step__n">2</span>
                    <span
                        ><span class="rad-step__title">{I18n.t("Configure your server")}</span>
                        <span class="rad-step__note"
                            >{I18n.t(
                                "Configure one file",
                            )}</span
                        ></span
                    >
                </div>
                <div class="rad-step">
                    <span class="rad-step__n">3</span>
                    <span
                        ><span class="rad-step__title"
                            >{I18n.t("Start your server in Nodecraft")}</span
                        >
                        <span class="rad-step__note"
                            >{I18n.t(
                                "That's it.",
                            )}</span
                        ></span
                    >
                </div>
            </div>

            <span class="rad-label rad-rise" style="--d: 320; display: block; margin-top: 28px">
                {I18n.t("Other options")}
            </span>
            <div class="rad-choices rad-rise" style="--d: 350">
                <button class="rad-choice" onclick={onguide}>
                    <span>
                        <span class="rad-choice__title">{I18n.t("I want to run my own")}</span>
                        <span class="rad-choice__note">
                            {I18n.t(
                                "On your PC, a VPS, or a home server. The guide shows how to install the server and the mod.",
                            )}
                        </span>
                    </span>
                    <span class="rad-choice__action">{I18n.t("Install guide →")}</span>
                </button>
                <button class="rad-choice" onclick={oncopyinvite}>
                    <span>
                        <span class="rad-choice__title">{I18n.t("A friend runs the server")}</span>
                        <span class="rad-choice__note">
                            {I18n.t(
                                "They only have to do it once. Copy a short message with the guide link and send it over.",
                            )}
                        </span>
                    </span>
                    <span class="rad-choice__action">{I18n.t("Copy message →")}</span>
                </button>
            </div>

            <div class="rad-swatchrow" style="margin-top: 22px">
                <button class="rad-pill-link" onclick={onwiki}>
                    {I18n.t("Wiki")} <span class="rad-pill-link__ext">&#8599;</span>
                </button>
                <button class="rad-pill-link" onclick={ondiscord}>
                    Discord <span class="rad-pill-link__ext">&#8599;</span>
                </button>
                <button class="rad-pill-link" onclick={onsite}>
                    bedrockvoicechat.com <span class="rad-pill-link__ext">&#8599;</span>
                </button>
            </div>
        </div>
    </div>

    {#snippet footbar()}
        <button class="rad-btn rad-btn--quiet" onclick={onintro}>{I18n.t("← Introduction")}</button
        >
        <button class="rad-btn rad-btn--lg" onclick={onsignin}
            >{I18n.t("I have an address — sign in")}</button
        >
    {/snippet}
</RadScreen>
