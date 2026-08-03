<script lang="ts">
    import Ring from "$radial/components/Ring.svelte";
    import type { RingSource } from "$radial/core/ring/RingSource";
    import { AnimationLoop } from "$radial/core/canvas/AnimationLoop";
    import { onDestroy } from "svelte";
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
        oninput: (value: string) => void;
        onconnect: () => void;
        onprivacy: () => void;
        onrevisit: () => void;
    }
    let {
        address,
        verdict,
        isCode = false,
        appVersion,
        oninput,
        onconnect,
        onprivacy,
        onrevisit,
    }: Props = $props();

    // One source, breathing: the address resolving. It goes quiet the moment the field
    // changes and comes back when a name resolves, so the ring reads as a readout
    // rather than decoration.
    let sources = $state<RingSource[]>([]);

    $effect(() => {
        if (verdict.state !== "ok") {
            sources = [];
            return;
        }
        return AnimationLoop.shared().add((t) => {
            sources = [
                {
                    angle: -Math.PI / 2 + 0.5,
                    volume: 0.72 + 0.28 * Math.abs(Math.sin(t * 0.0022)),
                    hue: "#ad76f7",
                },
            ];
        });
    });

    let resolveClass = $derived(
        verdict.state === "ok"
            ? "rad-resolve rad-resolve--ok"
            : verdict.state === "bad"
              ? "rad-resolve rad-resolve--bad"
              : "rad-resolve",
    );

    onDestroy(() => {
        sources = [];
    });
</script>

<RadScreen label="Connect">
    <div class="rad-split">
        <div class="rad-visual-pane">
            <div class="rad-visual">
                <Ring mode={verdict.ring} {sources} class="rad-ring--fill" />
                <span class="rad-caption">
                    <span class="rad-label">Acquiring</span>
                    <span class="rad-caption__value">{verdict.caption}</span>
                </span>
            </div>
        </div>
        <div class="rad-content-pane">
            <span class="rad-label rad-rise" style="--d: 50">Connect</span>
            <h2
                class="rad-display rad-rise"
                style="--d: 120; margin-top: 12px; font-size: 2.1111rem"
            >
                Which server are you&nbsp;joining?
            </h2>
            <div class="rad-rise" style="--d: 210; margin-top: 20px; max-width: 370px">
                <span class="rad-label">Server address</span>
                <div class="rad-field">
                    <span class="rad-field__prefix">ADDR</span>
                    <input
                        type="text"
                        value={address}
                        spellcheck="false"
                        autocapitalize="none"
                        autocorrect="off"
                        autocomplete="url"
                        aria-label="Server address"
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
                        Continue with a code
                    </button>
                {:else}
                    <button class="rad-ms-signin" onclick={onconnect}>
                        <span class="rad-ms-mark"><i></i><i></i><i></i><i></i></span>Sign in with
                        Microsoft
                    </button>
                {/if}
            </div>
        </div>
    </div>

    {#snippet footbar()}
        <span class="rad-footbar__actions">
            <button class="rad-btn rad-btn--quiet" onclick={onprivacy}>Privacy notice</button>
            <button class="rad-btn rad-btn--quiet" onclick={onrevisit}>What is this?</button>
        </span>
        <span class="rad-label rad-num">v{appVersion}</span>
    {/snippet}
</RadScreen>
