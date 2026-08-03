<script lang="ts">
    import Loader from "$radial/components/Loader.svelte";
    import Ring from "$radial/components/Ring.svelte";
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

    // Immediate rather than after four seconds: the OS prompt is already on screen
    // and the app looks frozen behind it.
    const PHRASES = [
        "Opening the permission prompt…",
        "Waiting for your response…",
        "Confirming microphone access…",
        "Almost there…",
    ];
</script>

<RadScreen label="Microphone">
    {#snippet topbar()}
        <StepDots {step} {total} />
    {/snippet}

    <div class="rad-split">
        <div class="rad-visual-pane">
            {#if state === "requesting"}
                <Loader loading={true} phrases={PHRASES} slowAfterSeconds={0} />
            {:else}
                <div class="rad-visual">
                    <Ring mode={state === "denied" ? "empty" : "lock"} class="rad-ring--fill" />
                    <span class="rad-caption">
                        <span class="rad-label">Input</span>
                        <span class="rad-caption__value">
                            {state === "denied" ? "ACCESS REFUSED" : "AWAITING PERMISSION"}
                        </span>
                    </span>
                </div>
            {/if}
        </div>
        <div class="rad-content-pane">
            <span class="rad-label rad-rise" style="--d: 50">01 &middot; Microphone</span>
            <h2 class="rad-display rad-rise" style="--d: 120; margin-top: 12px; font-size: 2rem">
                BVC needs<br /><b>your microphone.</b>
            </h2>
            <p class="rad-body rad-rise" style="--d: 210">
                Nothing is recorded or sent anywhere until you are in a voice session, and the
                noise gate keeps silence off the wire entirely.
            </p>

            {#if state === "denied"}
                <div class="rad-callout rad-rise" style="--d: 280; margin-top: 22px">
                    <span class="rad-choice__title">Microphone access was refused</span>
                    <span class="rad-choice__note">
                        Grant it in your system settings, then come back and try again. Voice chat
                        cannot work without it.
                    </span>
                </div>
                <div class="rad-rise" style="--d: 340; margin-top: 18px">
                    <button class="rad-btn rad-btn--lg rad-btn--primary" onclick={onrequest}>
                        Allow microphone access
                    </button>
                </div>
            {:else if state !== "requesting"}
                <div class="rad-choices rad-rise" style="--d: 300">
                    <button class="rad-choice" onclick={onrequest}>
                        <span>
                            <span class="rad-choice__title">Allow microphone access</span>
                            <span class="rad-choice__note">
                                Your operating system will ask. BVC never listens outside a session.
                            </span>
                        </span>
                        <span class="rad-choice__action">Continue &rarr;</span>
                    </button>
                </div>
            {/if}
        </div>
    </div>

    {#snippet footbar()}
        <span class="rad-label">Voice chat cannot work without a microphone</span>
        <span class="rad-label">Notifications and devices come next</span>
    {/snippet}
</RadScreen>
