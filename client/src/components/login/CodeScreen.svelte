<script lang="ts">
    import ProximityRing from "$radial/components/ProximityRing.svelte";
    import RadScreen from "../shell/RadScreen.svelte";
    import type { CodeLoginInput } from "../../js/app/loginCode";

    interface Props {
        server: string;
        error: string;
        isSubmitting: boolean;
        appVersion: string;
        onsubmit: (input: CodeLoginInput) => void;
        onback: () => void;
    }
    let { server, error, isSubmitting, appVersion, onsubmit, onback }: Props = $props();

    let code = $state("");

    function submit(event: Event): void {
        event.preventDefault();
        onsubmit({ code });
    }
</script>

<RadScreen label="Sign-in code">
    <div class="rad-split">
        <div class="rad-visual-pane">
            <div class="rad-visual">
                <ProximityRing mode="lock" class="rad-ring--fill" />
                <span class="rad-caption">
                    <span class="rad-label">{isSubmitting ? "Checking" : "Waiting"}</span>
                    <span class="rad-caption__value">
                        {isSubmitting ? "CHECKING YOUR CODE" : "READY"}
                    </span>
                </span>
            </div>
        </div>
        <div class="rad-content-pane">
            <span class="rad-label rad-rise" style="--d: 50">Sign in with a code</span>
            <h2
                class="rad-display rad-rise"
                style="--d: 120; margin-top: 12px; font-size: 1.8889rem"
            >
                Enter the code<br /><b>you were given.</b>
            </h2>
            <!--
              One field, because the code identifies the player and the game on its own.
              Anything else here would ask for something the server already knows and can
              only be contradicted.
            -->
            <form onsubmit={submit} class="rad-rise" style="--d: 200; margin-top: 24px; max-width: 400px">
                <span class="rad-label">Code</span>
                <div class="rad-field">
                    <span class="rad-field__prefix">CODE</span>
                    <!-- svelte-ignore a11y_autofocus -->
                    <input
                        type="text"
                        bind:value={code}
                        spellcheck="false"
                        autocapitalize="characters"
                        autocorrect="off"
                        autocomplete="one-time-code"
                        autofocus
                        aria-label="Code"
                    />
                </div>

                {#if error}
                    <div class="rad-resolve rad-resolve--bad" style="margin-top: 14px" role="alert">
                        <span aria-hidden="true">&#10005;</span>
                        <span>{error}</span>
                    </div>
                {/if}

                <button
                    class="rad-btn rad-btn--lg rad-btn--primary"
                    style="margin-top: 18px; width: 100%"
                    type="submit"
                    disabled={isSubmitting}
                >
                    Sign in
                </button>
                <button
                    class="rad-btn rad-btn--lg"
                    style="margin-top: 10px; width: 100%"
                    type="button"
                    onclick={onback}
                >
                    Back to sign in
                </button>
            </form>
        </div>
    </div>

    {#snippet footbar()}
        <!--
          The form's own Back sits below the field and its buttons. This one is in the
          chrome, so leaving is reachable from anywhere on the screen.
        -->
        <span class="rad-footbar__actions">
            <button class="rad-btn rad-btn--quiet" onclick={onback}>&larr; Back to sign in</button>
            <span class="rad-label">{server}</span>
        </span>
        <span class="rad-label rad-num">v{appVersion}</span>
    {/snippet}
</RadScreen>
