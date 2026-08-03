<script lang="ts">
    import Ring from "$radial/components/Ring.svelte";
    import { AnimationLoop } from "$radial/core/canvas/AnimationLoop";
    import { MarkData } from "$radial/core/mark/MarkData";
    import type { RingMode } from "$radial/bindings/RingBinding";
    import type { RingSource } from "$radial/core/ring/RingSource";
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

    const VOICES = [1, 8, 14].map((column, i) => ({
        hue: MarkData.hueAt(column),
        phase: i * 1.6,
    }));

    let sources = $state<RingSource[]>([]);

    /**
     * How alive the ring is, 0 to 1, eased toward its target each frame.
     *
     * A hard switch to `empty` cuts the colour and the amplitude in one frame, which
     * reads as a glitch rather than an answer. Fading the amplitude first and only
     * swapping the mode once the bars are already small makes it land as a decay.
     */
    let alive = $state(1);
    const DECAY_TARGET = 0.02;

    // Colour follows amplitude rather than leading it, so the drained palette arrives
    // after the bars have already gone quiet.
    let mode = $derived<RingMode>(
        alive < 0.12 ? "empty" : considering === "yes" ? "live" : "lock",
    );

    let caption = $derived(
        considering === "yes"
            ? "THREE PEOPLE TALKING"
            : considering === "no"
              ? "NOBODY IN RANGE"
              : "WAITING ON YOUR ANSWER",
    );

    $effect(() =>
        AnimationLoop.shared().add((t) => {
            // Exponential approach: quick out of the gate, settling rather than
            // arriving. Down faster than up, because "there is nobody there" should
            // land immediately and coming back to life can afford to bloom.
            const target = considering === "no" ? DECAY_TARGET : 1;
            const rate = target < alive ? 0.22 : 0.12;
            alive += (target - alive) * rate;

            if (alive <= DECAY_TARGET + 0.001) {
                sources = [];
                return;
            }

            if (considering === "yes") {
                sources = VOICES.map((v, i) => ({
                    angle: -Math.PI / 2 + i * ((Math.PI * 2) / 3) + Math.sin(t * 0.0005 + i) * 0.4,
                    volume:
                        (0.55 + 0.45 * Math.abs(Math.sin(t * 0.0018 + v.phase))) * alive,
                    hue: v.hue,
                }));
                return;
            }

            sources = [
                {
                    angle: -Math.PI / 2,
                    volume: (0.42 + 0.28 * Math.abs(Math.sin(t * 0.0011))) * alive,
                    hue: "#bb8dfa",
                },
            ];
        }),
    );
</script>

<RadScreen label="Server check">
    <div class="rad-split">
        <div class="rad-visual-pane">
            <div class="rad-visual">
                <Ring {mode} {sources} class="rad-ring--fill" />
                <span class="rad-caption">
                    <span class="rad-label">Your world</span>
                    <span class="rad-caption__value">{caption}</span>
                </span>
            </div>
        </div>

        <div class="rad-content-pane">
            <span class="rad-label rad-rise" style="--d: 50">Before you sign in</span>
            <h2 class="rad-display rad-rise" style="--d: 120; margin-top: 12px; font-size: 2rem">
                Is a BVC server already set up for <b>your world?</b>
            </h2>
            <p class="rad-body rad-rise" style="--d: 200">
                Your world needs a BVC server running somewhere before anyone can talk. If that is
                already done, all you need is its address.
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
                        <span class="rad-label">Yes &middot; I have an address</span>
                        <span class="rad-choice__title">Someone already set it up</span>
                        <span class="rad-choice__note">
                            A BVC server is running for your world and you have its address. Sign in
                            with the Microsoft account you play Minecraft with.
                        </span>
                    </span>
                    <span class="rad-choice__action">Sign in &rarr;</span>
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
                        <span class="rad-label">No &middot; or I am not sure</span>
                        <span class="rad-choice__title">Nobody has set it up yet</span>
                        <span class="rad-choice__note">
                            Voice chat needs a BVC server plus a mod on the world. It's a one-time
                            install &mdash; see what's involved.
                        </span>
                    </span>
                    <span class="rad-choice__action">Show me how &rarr;</span>
                </button>
            </div>
        </div>
    </div>

    {#snippet footbar()}
        <span class="rad-label">BVC server + world mod &middot; installed once</span>
        <span class="rad-label">Device setup continues after sign-in</span>
    {/snippet}
</RadScreen>
