<script lang="ts">
  import { I18n } from "$lib/i18n";
    import ProximityRing from "$radial/components/ProximityRing.svelte";
    import type { RingMode } from "$radial/bindings/RingBinding";
    import RadScreen from "../shell/RadScreen.svelte";

    interface Props {
        onhaveserver: () => void;
        onnoserver: () => void;
    }
    let { onhaveserver, onnoserver }: Props = $props();

    /**
     * Which answer is being considered, or null while the question is open.
     *
     * Set from focus as well as hover: the ring is the only feedback either option
     * gives, and a keyboard user is owed it too.
     */
    let considering = $state<"yes" | "no" | null>(null);

    /**
     * The answer being considered, as a ring state. `ProximityRing` eases the voices out
     * before the colour follows, so switching reads as a decay rather than a cut.
     */
    let mode = $derived<RingMode>(
        considering === "yes" ? "live" : considering === "no" ? "empty" : "lock",
    );

    let audible = $state(0);

    let caption = $derived(
        considering === "yes"
            ? `${audible} IN EARSHOT`
            : considering === "no"
              ? "NOBODY IN RANGE"
              : "WAITING ON YOUR ANSWER",
    );
</script>

<RadScreen label={I18n.t("Server check")}>
    <div class="rad-split">
        <div class="rad-visual-pane">
            <div class="rad-visual">
                <ProximityRing
                    {mode}
                    onaudible={(n) => (audible = n)}
                    class="rad-ring--fill"
                />
                <span class="rad-caption">
                    <span class="rad-label">{I18n.t("Your world")}</span>
                    <span class="rad-caption__value">{caption}</span>
                </span>
            </div>
        </div>

        <div class="rad-content-pane">
            <span class="rad-label rad-rise" style="--d: 50">{I18n.t("Before you sign in")}</span>
            <h2 class="rad-display rad-rise" style="--d: 120; margin-top: 12px; font-size: 2rem">
                {I18n.t("Is a BVC server already set up for")} <b>your world?</b>
            </h2>
            <p class="rad-body rad-rise" style="--d: 200">
                {I18n.t("Your world needs a BVC server running somewhere before anyone can talk. If that is already done, all you need is its address.")}
            </p>

            <div class="rad-choices rad-rise" style="--d: 290">
                <button
                    class="rad-choice"
                    onclick={onhaveserver}
                    onmouseenter={() => (considering = "yes")}
                    onmouseleave={() => (considering = null)}
                    onfocus={() => (considering = "yes")}
                    onblur={() => (considering = null)}
                >
                    <span>
                        <span class="rad-label">{I18n.t("Yes · I have an address")}</span>
                        <span class="rad-choice__title">{I18n.t("Someone already set it up")}</span>
                        <span class="rad-choice__note">
                            {I18n.t("A BVC server is running for your world and you have its address. Sign in with the Microsoft account you play Minecraft with.")}
                        </span>
                    </span>
                    <span class="rad-choice__action">{I18n.t("Sign in →")}</span>
                </button>

                <button
                    class="rad-choice"
                    onclick={onnoserver}
                    onmouseenter={() => (considering = "no")}
                    onmouseleave={() => (considering = null)}
                    onfocus={() => (considering = "no")}
                    onblur={() => (considering = null)}
                >
                    <span>
                        <span class="rad-label">{I18n.t("No · or I am not sure")}</span>
                        <span class="rad-choice__title">{I18n.t("Nobody has set it up yet")}</span>
                        <span class="rad-choice__note">
                            {I18n.t("Voice chat needs a BVC server plus a mod on the world. It's a one-time install — see what's involved.")}
                        </span>
                    </span>
                    <span class="rad-choice__action">{I18n.t("Show me how →")}</span>
                </button>
            </div>
        </div>
    </div>

    {#snippet footbar()}
        <span class="rad-label">{I18n.t("BVC server + world mod · installed once")}</span>
        <span class="rad-label">{I18n.t("Device setup continues after sign-in")}</span>
    {/snippet}
</RadScreen>
