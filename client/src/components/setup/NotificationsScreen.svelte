<script lang="ts">
  import { I18n } from "$lib/i18n";
    import Loader from "$radial/components/Loader.svelte";
    import ProximityRing from "$radial/components/ProximityRing.svelte";
    import type { PermissionFlowState } from "../../js/app/PermissionRequestManager";
    import RadScreen from "../shell/RadScreen.svelte";
    import StepDots from "../shell/StepDots.svelte";

    interface Props {
        state: PermissionFlowState;
        step: number;
        total: number;
        onrequest: () => void;
    }
    let { state, step, total, onrequest }: Props = $props();

    const PHRASES = [
        "Opening the permission prompt…",
        "Waiting for your response…",
        "Confirming notification access…",
        "Almost there…",
    ];
</script>

<RadScreen label={I18n.t("Notifications")}>
    {#snippet topbar()}
        <StepDots {step} {total} />
    {/snippet}

    <div class="rad-split">
        <div class="rad-visual-pane">
            {#if state === "requesting"}
                <Loader loading={true} phrases={PHRASES} slowAfterSeconds={0} />
            {:else}
                <div class="rad-visual">
                    <ProximityRing
                        mode={state === "denied" ? "empty" : "lock"}
                        class="rad-ring--fill"
                    />
                    <span class="rad-caption">
                        <span class="rad-label">{I18n.t("Alerts")}</span>
                        <span class="rad-caption__value">
                            {state === "denied" ? "ACCESS REFUSED" : "AWAITING PERMISSION"}
                        </span>
                    </span>
                </div>
            {/if}
        </div>
        <div class="rad-content-pane">
            <span class="rad-label rad-rise" style="--d: 50">02 · Notifications</span>
            <h2 class="rad-display rad-rise" style="--d: 120; margin-top: 12px; font-size: 2rem">
                {I18n.t("Voice has to keep running")}<br /><b>when you switch to the game.</b>
            </h2>
            <!--
              The reason is the permission itself, not the alerts. Both platforms require
              a notification to keep audio alive once the app is off screen, and playing
              Minecraft means BVC is always off screen — so this is what stops voice being
              cut the moment someone starts playing.
            -->
            <p class="rad-body rad-rise" style="--d: 210">
                {I18n.t("Android and iOS both require a notification to let an app hold the microphone in the background. Without it your voice cuts out the moment BVC leaves the screen — which is every moment you are actually playing.")}
            </p>

            {#if state === "denied"}
                <div class="rad-callout rad-rise" style="--d: 280; margin-top: 22px">
                    <span class="rad-choice__title">{I18n.t("Notification access was refused")}</span>
                    <span class="rad-choice__note">
                        {I18n.t("Grant it in your system settings, then come back and try again. Voice cannot survive leaving the screen without it.")}
                    </span>
                </div>
                <div class="rad-rise" style="--d: 340; margin-top: 18px">
                    <button class="rad-btn rad-btn--lg rad-btn--primary" onclick={onrequest}>
                        {I18n.t("Allow notifications")}
                    </button>
                </div>
            {:else if state !== "requesting"}
                <div class="rad-choices rad-rise" style="--d: 300">
                    <button class="rad-choice" onclick={onrequest}>
                        <span>
                            <span class="rad-choice__title">{I18n.t("Allow notifications")}</span>
                            <span class="rad-choice__note">
                                {I18n.t("One quiet ongoing notification while you are in a session, plus people arriving and channels you are part of.")}
                            </span>
                        </span>
                        <span class="rad-choice__action">{I18n.t("Continue →")}</span>
                    </button>
                </div>
            {/if}
        </div>
    </div>

    {#snippet footbar()}
        <span class="rad-label">{I18n.t("Required · background audio depends on it")}</span>
        <span class="rad-label">{I18n.t("Devices come next")}</span>
    {/snippet}
</RadScreen>
