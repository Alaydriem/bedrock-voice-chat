<script lang="ts">
    import { I18n } from "#lib/i18n/index.js";
    import Icon from "$radial/components/Icon.svelte";
    import RadScreen from "../shell/RadScreen.svelte";

    type Prerequisite = "domain" | "cloudflare" | "server";

    interface Props {
        onsignup: (prerequisitesChecked: number) => void;
        ontoggle: (item: Prerequisite, checked: boolean) => void;
        onguide: (guide: "bedrock" | "java") => void;
        ondiscord: () => void;
        onback: () => void;
        onsignin: () => void;
    }
    let { onsignup, ontoggle, onguide, ondiscord, onback, onsignin }: Props = $props();

    const prerequisites: { item: Prerequisite; label: string; note: string }[] = [
        {
            item: "domain",
            label: I18n.t("A domain name"),
            note: I18n.t("From any registrar. An IP address does not work."),
        },
        {
            item: "cloudflare",
            label: I18n.t("A Cloudflare account"),
            note: I18n.t("Free plan works"),
        },
        {
            item: "server",
            label: I18n.t("A Nodecraft Pro server"),
            note: I18n.t("Bedrock Dedicated Server"),
        },
    ];

    let ticked = $state<Record<Prerequisite, boolean>>({
        domain: false,
        cloudflare: false,
        server: false,
    });

    function toggle(item: Prerequisite): void {
        ticked[item] = !ticked[item];
        ontoggle(item, ticked[item]);
    }

    function signup(): void {
        onsignup(Object.values(ticked).filter(Boolean).length);
    }
</script>

<RadScreen label={I18n.t("Run it on Nodecraft")}>
    <div class="rad-split">
        <div class="rad-visual-pane">
            <div class="rad-visual">
                <div class="rad-matrix">
                    <div class="rad-matrix__group">
                        <div class="rad-matrix__head">{I18n.t("You need")}</div>
                        {#each prerequisites as row (row.item)}
                            <button
                                class="rad-checkbox"
                                role="checkbox"
                                aria-checked={ticked[row.item]}
                                onclick={() => toggle(row.item)}
                            >
                                <span class="rad-checkbox__box"><Icon name="check" /></span>
                                <span class="rad-checkbox__label">{row.label}</span>
                                <span class="rad-checkbox__note">{row.note}</span>
                            </button>
                        {/each}
                    </div>
                    <p class="rad-matrix__note">
                        <span
                            >{I18n.t(
                                "BVC uses your domain and Cloudflare to get a TLS certificate.",
                            )}</span
                        >
                    </p>
                </div>
                <span class="rad-caption">
                    <span class="rad-label">{I18n.t("You need")}</span>
                    <span class="rad-caption__value">{I18n.t("3 THINGS")}</span>
                </span>
            </div>
        </div>
        <div class="rad-content-pane rad-content-pane--top">
            <span class="rad-label rad-rise" style="--d: 50">{I18n.t("Run it on Nodecraft")}</span>
            <h2 class="rad-display rad-rise" style="--d: 120; margin-top: 12px; font-size: 2rem">
                {I18n.t("Before you start,")}<br /><b>{I18n.t("you need three things.")}</b>
            </h2>
            <p class="rad-body rad-rise" style="--d: 200">
                {I18n.t(
                    "Nodecraft runs your Bedrock server and BVC on the same machine. Get these three things, then follow the guide.",
                )}
            </p>

            <div class="rad-callout rad-rise" style="--d: 320">
                <span>
                    <b>{I18n.t("Get 30% off your first month on Nodecraft.")}</b>
                    {I18n.t("Signing up with this link also helps fund BVC.")}
                </span>
            </div>
            <button
                class="rad-btn rad-btn--primary rad-btn--lg rad-rise"
                style="--d: 350; margin-top: 14px"
                onclick={signup}>{I18n.t("Get a Nodecraft server →")}</button
            >

            <span class="rad-label rad-rise" style="--d: 380; display: block; margin-top: 28px">
                {I18n.t("Then set up BVC")}
            </span>
            <div class="rad-steps rad-rise" style="--d: 410">
                <div class="rad-step">
                    <span class="rad-step__n">1</span>
                    <span
                        ><span class="rad-step__title"
                            >{I18n.t("Move your domain's DNS to Cloudflare")}</span
                        >
                        <span class="rad-step__note">{I18n.t("Then create an API token for BVC")}</span
                        ></span
                    >
                </div>
                <div class="rad-step">
                    <span class="rad-step__n">2</span>
                    <span
                        ><span class="rad-step__title"
                            >{I18n.t("Install BVC on your Nodecraft server")}</span
                        >
                        <span class="rad-step__note"
                            >{I18n.t("Follow the Bedrock on Nodecraft guide")}</span
                        ></span
                    >
                </div>
                <div class="rad-step">
                    <span class="rad-step__n">3</span>
                    <span
                        ><span class="rad-step__title"
                            >{I18n.t("Sign in with your server address")}</span
                        ></span
                    >
                </div>
            </div>

            <div class="rad-swatchrow" style="margin-top: 22px">
                <button class="rad-pill-link" onclick={() => onguide("bedrock")}>
                    {I18n.t("Bedrock on Nodecraft guide")}
                    <span class="rad-pill-link__ext">&#8599;</span>
                </button>
                <button class="rad-pill-link" onclick={() => onguide("java")}>
                    {I18n.t("Java guides")} <span class="rad-pill-link__ext">&#8599;</span>
                </button>
                <button class="rad-pill-link" onclick={ondiscord}>
                    Discord <span class="rad-pill-link__ext">&#8599;</span>
                </button>
            </div>
        </div>
    </div>

    {#snippet footbar()}
        <button class="rad-btn rad-btn--quiet" onclick={onback}>{I18n.t("← Other options")}</button>
        <button class="rad-btn rad-btn--lg" onclick={onsignin}
            >{I18n.t("I have an address — sign in")}</button
        >
    {/snippet}
</RadScreen>
