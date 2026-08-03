<script lang="ts">
    import Ring from "$radial/components/Ring.svelte";
    import LevelMeter from "$radial/components/LevelMeter.svelte";
    import { AnimationLoop } from "$radial/core/canvas/AnimationLoop";
    import { MarkData } from "$radial/core/mark/MarkData";
    import { PositionalSource } from "$radial/core/sources/PositionalSource";
    import { TimelineBinding } from "$radial/bindings/TimelineBinding";
    import { StepFlow } from "$radial/core/controllers/StepFlow";
    import type { RingSource } from "$radial/core/ring/RingSource";
    import RadScreen from "../shell/RadScreen.svelte";
    import StepDots from "../shell/StepDots.svelte";

    interface Props {
        step: number;
        onstep: (step: number) => void;
        onnext: () => void;
        onback: () => void;
        onskip: () => void;
    }
    let { step, onstep, onnext, onback, onskip }: Props = $props();

    const TOTAL = 4;

    const CAPTIONS = [
        "Four things worth knowing before you sign in",
        "Proximity keeps running underneath channels",
        "No special hosting, no separate accounts",
        "Everything here works on desktop and phone",
    ];

    /**
     * Eight people, placed on the ring and moving.
     *
     * Synthetic on purpose: the introduction runs before there is a server, so there
     * is no real data to show. Hues come from columns of the mark, so a player's
     * colour is provably part of the palette rather than picked next to it.
     *
     * Phases are spread across the whole cycle and radii stay inside the audible
     * range. Both matter: a cast that falls silent together, or wanders past 80 m,
     * makes the screen that sells proximity look like it has broken.
     */
    /**
     * The four on step one are close, and that is load-bearing rather than cosmetic.
     * Falloff is quadratic, so a player at 0.7 of range contributes (1 - 0.7)² = 0.09
     * — under the meter's own 0.08 speaking threshold, which greys the row out. Anyone
     * meant to look audible has to actually be near.
     */
    const ROSTER = [
        { name: "ALAYDRIEM", hue: MarkData.hueAt(1), bearing: 0.55, drift: 0.00040, radius: 0.14, breathe: 0.00009 },
        { name: "PETRA", hue: MarkData.hueAt(4), bearing: 1.8, drift: -0.00029, radius: 0.22, breathe: 0.00006 },
        { name: "JUNO", hue: MarkData.hueAt(8), bearing: 3.0, drift: 0.00022, radius: 0.3, breathe: 0.00004 },
        { name: "MARROW", hue: MarkData.hueAt(11), bearing: 4.2, drift: -0.00018, radius: 0.38, breathe: 0.00007 },
        { name: "VESPER", hue: MarkData.hueAt(14), bearing: 5.4, drift: 0.00031, radius: 0.26, breathe: 0.00005 },
        { name: "CASS", hue: MarkData.hueAt(17), bearing: 2.4, drift: 0.00026, radius: 0.34, breathe: 0.00008 },
        { name: "RILEY", hue: MarkData.hueAt(19), bearing: 4.9, drift: -0.00024, radius: 0.2, breathe: 0.00005 },
        { name: "ODEN", hue: MarkData.hueAt(22), bearing: 1.1, drift: 0.00019, radius: 0.42, breathe: 0.00006 },
    ];

    /**
     * Step one shows four, not the whole roster: five rows of readout overflowed the
     * pane at shorter window heights. Step two shows everyone, because a channel with
     * a couple of people in it reads as empty and teaches the opposite of the point.
     */
    const NEARBY = ROSTER.slice(0, 4);

    /**
     * A voice that is always saying something.
     *
     * `SyntheticLevelSource` gates half of its cycle to exact silence, which is
     * honest for a roster but wrong here: below 0.03 a voice leaves the ring
     * entirely and its row greys out, so a screen meant to show constant activity
     * spent half its time looking dead. Same two-sine shape as the mark's own dance,
     * floored so it never reaches zero.
     */
    function voice(t: number, index: number): number {
        const phase = (index * Math.PI * 2) / ROSTER.length;
        const a = 0.5 + 0.5 * Math.sin(t * 0.0027 + phase);
        const b = 0.5 + 0.5 * Math.sin(t * 0.0011 + phase * 1.7);
        return 0.34 + 0.66 * (a * 0.62 + b * 0.38);
    }

    let placements = $state<RingSource[]>([]);
    let audible = $state(0);
    let levels = $state(NEARBY.map(() => 0));
    let readouts = $state(NEARBY.map(() => ({ live: false, gain: "––", range: "–" })));

    $effect(() =>
        AnimationLoop.shared().add((t) => {
            if (step !== 1) return;
            const next: RingSource[] = [];
            const nextLevels: number[] = [];
            const rows = NEARBY.map((p, i) => {
                const distance =
                    Math.max(0.08, Math.min(0.55, p.radius + Math.sin(t * p.breathe + i) * 0.08)) *
                    PositionalSource.RANGE;
                const source = PositionalSource.toRingSource(
                    { bearing: p.bearing + t * p.drift, distance, hue: p.hue },
                    voice(t, i),
                );
                // The meter shows the player's own voice, exactly as the channel step
                // does: raw, floored, always moving. Falloff belongs to the ring and to
                // the percentage — how loud they are *to you* — not to the waveform,
                // which greyed out the moment distance pushed it under the meter's
                // 0.08 speaking threshold.
                const own = voice(t, i);
                const volume = source?.volume ?? 0;
                if (source) next.push(source);
                nextLevels.push(own);
                return {
                    live: volume > 0,
                    gain: `${Math.round(volume * 100)}%`,
                    range: `${Math.round(distance)} m`,
                };
            });
            placements = next;
            levels = nextLevels;
            readouts = rows;
            audible = next.length;
        }),
    );

    // Channels are full volume at any distance, which is the entire point of them, so
    // these carry no falloff.
    let channelLevels = $state(ROSTER.map(() => 0));
    $effect(() => {
        if (step !== 2) return;
        return AnimationLoop.shared().add((t) => {
            channelLevels = ROSTER.map((_, i) => voice(t, i));
        });
    });

    /**
     * Membership moves on its own, because the thing being explained is that proximity
     * keeps running underneath a channel rather than being replaced by it.
     *
     * Both sides stay populated. A channel holding one person reads as empty, which
     * teaches the opposite of what this step is for.
     */
    const MIN_PER_CHANNEL = 3;
    let assignment = $state([1, 1, 1, 1, 0, 0, 0, 0]);
    let swapSeed = 2600;
    $effect(() => {
        if (step !== 2) return;
        const id = setInterval(() => {
            swapSeed += 1;
            const i = Math.floor(Math.abs(Math.sin(swapSeed * 12.9898)) * ROSTER.length) % ROSTER.length;
            const next = [...assignment];
            next[i] = next[i] === 1 ? 0 : 1;
            const inChannel = next.filter((c) => c === 1).length;
            if (inChannel < MIN_PER_CHANNEL || ROSTER.length - inChannel < MIN_PER_CHANNEL) return;
            assignment = next;
        }, 3200);
        return () => clearInterval(id);
    });

    let timelineCanvas: HTMLCanvasElement | undefined = $state();
    $effect(() => {
        if (step !== 4 || !timelineCanvas) return;
        const binding = new TimelineBinding(timelineCanvas, {
            lanes: NEARBY.map((p) => ({ name: p.name, hue: p.hue })),
        });
        return () => binding.destroy();
    });

    let recTime = $state("04:12");
    $effect(() => {
        if (step !== 4) return;
        return AnimationLoop.shared().add((t) => {
            const seconds = 252 + (Math.floor(t / 1000) % 600);
            recTime = `${String(Math.floor(seconds / 60)).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`;
        });
    });

    // The entry stagger has to replay when the step changes, or steps two to four
    // arrive with no entrance at all.
    let body: HTMLElement | undefined = $state();
    $effect(() => {
        void step;
        if (body) StepFlow.restartStagger(body);
    });

    // Carries the index so a member's meter reads the same level as its ring bar.
    function members(channel: number) {
        return ROSTER.map((player, index) => ({ player, index })).filter(
            (entry) => assignment[entry.index] === channel,
        );
    }
</script>

<RadScreen>
    {#snippet topbar()}
        <StepDots {step} total={TOTAL} onselect={onstep} />
    {/snippet}

    <div class="rad-split">
        <div class="rad-visual-pane">
            {#if step === 1}
                <div class="rad-visual">
                    <Ring mode="live" sources={placements} class="rad-ring--fill" />
                    <span class="rad-caption">
                        <span class="rad-label">Your range</span>
                        <span class="rad-caption__value">80 M &middot; {audible} IN EARSHOT</span>
                    </span>
                </div>
            {:else if step === 2}
                <div class="rad-visual">
                    <div class="rad-channels">
                        {#each [0, 1] as channel (channel)}
                            <div
                                class="rad-channel {channel === 1 && members(1).length > 0 ? 'is-active' : ''}"
                            >
                                <div class="rad-channel__head">
                                    <span>{channel === 0 ? "Proximity" : "Raid party"}</span>
                                    <span>{members(channel).length}</span>
                                </div>
                                <div>
                                    {#each members(channel) as entry (entry.player.name)}
                                        <div class="rad-channel__member">
                                            <LevelMeter
                                                level={channelLevels[entry.index] *
                                                    (channel === 1 ? 1 : 0.82)}
                                                color={entry.player.hue}
                                                cell={3}
                                            />
                                            <span class="rad-channel__member-name">
                                                {entry.player.name}
                                            </span>
                                        </div>
                                    {:else}
                                        <div class="rad-channel__empty">&mdash; nobody here &mdash;</div>
                                    {/each}
                                </div>
                            </div>
                        {/each}
                    </div>
                    <span class="rad-caption">
                        <span class="rad-label">Channel</span>
                        <span class="rad-caption__value">FULL VOLUME &middot; ANY DISTANCE</span>
                    </span>
                </div>
            {:else if step === 3}
                <div class="rad-visual">
                    <div class="rad-matrix">
                        <div class="rad-matrix__group">
                            <div class="rad-matrix__head">
                                Where your world can live &mdash; the mod goes here
                            </div>
                            <div class="rad-matrix__row">
                                <span class="rad-matrix__blocks"><i style="background:#8239d8"></i><i style="background:#6a50e9"></i><i style="background:#466cf3"></i></span>
                                <span>Your own server</span><span class="rad-matrix__tag rad-matrix__tag--yes">Supported</span>
                            </div>
                            <div class="rad-matrix__row">
                                <span class="rad-matrix__blocks"><i style="background:#28bae1"></i><i style="background:#21d8d8"></i><i style="background:#26ddcd"></i></span>
                                <span>Aternos &mdash; free hosting</span><span class="rad-matrix__tag rad-matrix__tag--yes">Supported</span>
                            </div>
                            <div class="rad-matrix__row">
                                <span class="rad-matrix__blocks"><i style="background:#34d8a0"></i><i style="background:#3bd869"></i><i style="background:#6fd846"></i></span>
                                <span>Minecraft Realms</span><span class="rad-matrix__tag rad-matrix__tag--yes">Supported</span>
                            </div>
                            <div class="rad-matrix__row">
                                <span class="rad-matrix__blocks"><i style="background:#aee236"></i><i style="background:#f8e433"></i><i style="background:#f9bf21"></i></span>
                                <span>Java + Geyser &amp; Floodgate</span><span class="rad-matrix__tag rad-matrix__tag--yes">Supported</span>
                            </div>
                        </div>
                        <div class="rad-matrix__group">
                            <div class="rad-matrix__head">Who can join</div>
                            <div class="rad-matrix__row">
                                <span class="rad-matrix__blocks"><i style="background:#8239d8"></i><i style="background:#6a50e9"></i><i style="background:#466cf3"></i></span>
                                <span>Windows &middot; macOS &middot; Linux</span><span class="rad-matrix__tag rad-matrix__tag--yes">Ready</span>
                            </div>
                            <div class="rad-matrix__row">
                                <span class="rad-matrix__blocks"><i style="background:#28bae1"></i><i style="background:#21d8d8"></i><i style="background:#26ddcd"></i></span>
                                <span>Android &middot; iOS</span><span class="rad-matrix__tag rad-matrix__tag--yes">Ready</span>
                            </div>
                            <div class="rad-matrix__row">
                                <span class="rad-matrix__blocks"><i style="background:#34d8a0"></i><i style="background:#3bd869"></i><i style="background:#6fd846"></i></span>
                                <span>Xbox &middot; PlayStation &middot; Switch</span><span class="rad-matrix__tag rad-matrix__tag--yes">Via phone</span>
                            </div>
                        </div>
                        <p class="rad-matrix__note">
                            <span>The BVC server runs separately, on <b>any machine you control</b> &mdash;
                                your gaming PC, a VPS, or a home box.</span>
                        </p>
                    </div>
                    <span class="rad-caption">
                        <span class="rad-label">Reach</span>
                        <span class="rad-caption__value">YOUR WORLD STAYS WHERE IT IS</span>
                    </span>
                </div>
            {:else}
                <div class="rad-visual">
                    <div class="rad-timeline">
                        <div class="rad-timeline__ruler">
                            <span class="rad-timeline__rec"><i></i>REC <span class="rad-num">{recTime}</span></span>
                            <span class="rad-timeline__marks">
                                <span>00:00</span><span>01:24</span><span>02:48</span><span>04:12</span>
                            </span>
                        </div>
                        <div class="rad-timeline-lanes"><canvas bind:this={timelineCanvas}></canvas></div>
                    </div>
                    <span class="rad-caption">
                        <span class="rad-label">Session</span>
                        <span class="rad-caption__value">3 TRACKS &middot; TIMECODED</span>
                    </span>
                </div>
            {/if}
        </div>

        <div class="rad-content-pane" bind:this={body}>
            {#if step === 1}
                <span class="rad-label rad-rise" style="--d: 50">01 &middot; Proximity</span>
                <h2 class="rad-display rad-rise" style="--d: 120; margin-top: 12px">
                    Walk up to someone.<br /><b>You're already talking.</b>
                </h2>
                <p class="rad-body rad-rise" style="--d: 210">
                    Voices get louder as players come toward you and fade as they leave. No lobbies,
                    no invites, and nobody shouting over a call from three biomes away.
                </p>
                <div class="rad-chips rad-rise" style="--d: 300">
                    <span class="rad-chip"><i style="background: #21d8d8"></i>Positional audio</span>
                    <span class="rad-chip"><i style="background: #3bd869"></i>Whisper &amp; shout</span>
                    <span class="rad-chip"><i style="background: #aee236"></i>Spectator support</span>
                </div>
                <div class="rad-readout-list rad-rise" style="--d: 380">
                    {#each NEARBY as p, i (p.name)}
                        <div class="rad-readout {readouts[i].live ? 'is-live' : ''}">
                            <span class="rad-readout__dot" style="background:{p.hue}"></span>
                            <span>{p.name}</span>
                            <LevelMeter level={levels[i]} color={p.hue} cell={3} />
                            <span class="rad-readout__gain">{readouts[i].gain}</span>
                            <span class="rad-readout__range">{readouts[i].range}</span>
                        </div>
                    {/each}
                </div>
            {:else if step === 2}
                <span class="rad-label rad-rise" style="--d: 50">02 &middot; Channels</span>
                <h2 class="rad-display rad-rise" style="--d: 120; margin-top: 12px">
                    Split off into<br /><b>your own channel.</b>
                </h2>
                <p class="rad-body rad-rise" style="--d: 210">
                    Running something across the whole map? Drop into a channel and everyone stays
                    at full volume however far apart you get. Proximity keeps running underneath the
                    entire time.
                </p>
                <div class="rad-chips rad-rise" style="--d: 300">
                    <span class="rad-chip"><i style="background: #8239d8"></i>Persistent groups</span>
                    <span class="rad-chip"><i style="background: #28bae1"></i>Join from in-game</span>
                    <span class="rad-chip"><i style="background: #f9bf21"></i>Server-wide broadcast</span>
                    <span class="rad-chip"><i style="background: #3bd869"></i>Per-player volume 0&ndash;150%</span>
                    <span class="rad-chip"><i style="background: #f67414"></i>Moderator controls</span>
                </div>
            {:else if step === 3}
                <span class="rad-label rad-rise" style="--d: 50">03 &middot; Anywhere you play</span>
                <h2 class="rad-display rad-rise" style="--d: 120; margin-top: 12px">
                    It works with<br /><b>the world you already have.</b>
                </h2>
                <p class="rad-body rad-rise" style="--d: 210">
                    Your world stays exactly where it is. The mod goes on it; the BVC server runs
                    on a machine you control.
                </p>
                <div class="rad-steps rad-rise" style="--d: 300">
                    <div class="rad-step">
                        <span class="rad-step__n">1</span>
                        <span>
                            <span class="rad-step__title">Run the BVC server</span>
                            <span class="rad-step__note">Your PC, a VPS, or a home box</span>
                        </span>
                    </div>
                    <div class="rad-step">
                        <span class="rad-step__n">2</span>
                        <span>
                            <span class="rad-step__title">Add the mod to your world</span>
                            <span class="rad-step__note">A Bedrock add-on, or the Fabric or Paper mod for Java</span>
                        </span>
                    </div>
                    <div class="rad-step">
                        <span class="rad-step__n">3</span>
                        <span>
                            <span class="rad-step__title">Share the address and talk</span>
                            <span class="rad-step__note">Everyone signs in with the account they already play on</span>
                        </span>
                    </div>
                </div>
            {:else}
                <span class="rad-label rad-rise" style="--d: 50">04 &middot; Record &amp; control</span>
                <h2 class="rad-display rad-rise" style="--d: 120; margin-top: 12px">
                    Record everyone.<br /><b>On separate tracks.</b>
                </h2>
                <p class="rad-body rad-rise" style="--d: 210">
                    Every voice lands on its own timecoded track, ready to drop straight into the
                    edit &mdash; and the controls reach the desk beside your keyboard.
                </p>
                <div class="rad-chips rad-rise" style="--d: 300">
                    <span class="rad-chip"><i style="background: #21d8d8"></i>Split-track recording</span>
                    <span class="rad-chip"><i style="background: #aee236"></i>Stream Deck plugin</span>
                    <span class="rad-chip"><i style="background: #f9bf21"></i>Push-to-talk &amp; hotkeys</span>
                    <span class="rad-chip"><i style="background: #8239d8"></i>Noise gate &amp; gain</span>
                    <span class="rad-chip"><i style="background: #f67414"></i>WebSocket API</span>
                    <span class="rad-chip"><i style="background: #3bd869"></i>Export to WAV or MP4</span>
                </div>
            {/if}
        </div>
    </div>

    {#snippet footbar()}
        <span class="rad-label">{CAPTIONS[step - 1]}</span>
        <span class="rad-footbar__actions">
            <button class="rad-btn rad-btn--lg rad-btn--quiet" onclick={onskip}>Skip</button>
            {#if step > 1}
                <button class="rad-btn rad-btn--lg rad-btn--quiet" onclick={onback}>Back</button>
            {/if}
            <button class="rad-btn rad-btn--lg rad-btn--primary" onclick={onnext}>
                {step === TOTAL ? "Continue" : "Next"}
            </button>
        </span>
    {/snippet}
</RadScreen>
