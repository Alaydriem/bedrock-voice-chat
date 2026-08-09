<script lang="ts">
  import Icon from "./Icon.svelte";
  import LevelMeter from "./LevelMeter.svelte";
  import type { LevelSource } from "$radial/core/sources/LevelSource";
  import type { SelfSnapshot } from "$radial/core/controllers/SelfState";

  interface Props {
    name: string;
    /** Two letters over the brand colour. Yours is the one avatar that is not a hue. */
    initials?: string;
    state: SelfSnapshot;
    /** Your own mic level, after the noise gate. */
    source?: LevelSource;
    /** Group you are in, shown under your name. Empty for proximity only. */
    groupName?: string;
    /** Elapsed recording time as mm:ss. */
    recordTime?: string;
    onmute?: (e: MouseEvent) => void;
    ondeafen?: (e: MouseEvent) => void;
    onrecord?: (e: MouseEvent) => void;
    onhold?: (down: boolean) => void;
    onidentity?: () => void;
    /**
     * Tooltip on the identity button. Defaults to English: the kit cannot reach the
     * application's translation surface, so a caller that has one passes the words in.
     */
    identityLabel?: string;
    /**
     * Why the record button is unavailable, shown as its tooltip. Defaults to English
     * for the same reason as `identityLabel`.
     */
    recordBlockedLabel?: string;
    /** Renders the phone capsule instead of the desktop pill. */
    capsule?: boolean;
  }

  let {
    name,
    initials = name.slice(0, 2).toUpperCase(),
    state,
    source,
    groupName = "",
    recordTime = "00:00",
    onmute,
    ondeafen,
    onrecord,
    onhold,
    onidentity,
    identityLabel = "Profile and sign-out",
    recordBlockedLabel = "Recording is off on this server",
    capsule = false,
  }: Props = $props();

  // In push-to-talk the mic button is a hold control, not a toggle: not holding it
  // already is mute, so a separate mute would be a second word for the same thing.
  const ptt = $derived(state.mode === "ptt");

  /**
   * Muted is drawn in push-to-talk too.
   *
   * Hiding it read as an open microphone at rest, which is the opposite of what
   * push-to-talk means and the one thing a mute indicator must never get wrong. The label
   * carries the difference between "muted, press to talk" and a mute you have to undo.
   */
  const closed = $derived(state.muted && !state.holding);
</script>

<div class={capsule ? "rad-self-capsule" : "rad-self-pill"}>
  <span class="rad-self__avatar">{initials}</span>

  <button class="rad-self__id" type="button" onclick={onidentity} title={identityLabel}>
    <span class="rad-self__name">
      <span>{name}</span>
      <span class="rad-health-dot"></span>
      <span class="rad-self__chev">&#9660;</span>
    </span>
    <span class="rad-self__sub">
      <LevelMeter {source} cell={capsule ? 2 : 3} color={state.transmitting ? "rainbow" : "#7a68a0"} probe="self" />
      <span class="rad-self__state">{groupName}</span>
    </span>
  </button>

  <button
    class="rad-self__btn {capsule ? 'rad-self__btn--primary' : ''}"
    class:is-holding={ptt && state.holding}
    class:is-ptt={ptt}
    type="button"
    aria-pressed={state.muted}
    aria-label={ptt
      ? state.holding
        ? "Talking, release to stop"
        : "Muted. Hold to talk"
      : state.muted
        ? "Unmute"
        : "Mute"}
    onclick={(e) => !ptt && onmute?.(e)}
    onpointerdown={() => ptt && onhold?.(true)}
    onpointerup={() => ptt && onhold?.(false)}
    onpointerleave={() => ptt && onhold?.(false)}
    onpointercancel={() => ptt && onhold?.(false)}
  >
    <Icon name={closed ? "micoff" : "mic"} />
  </button>

  <button
    class="rad-self__btn rad-self__btn--deafen"
    type="button"
    aria-pressed={state.deafened}
    aria-label={state.deafened ? "Undeafen" : "Deafen"}
    onclick={ondeafen}
  >
    <Icon name={state.deafened ? "headoff" : "head"} />
  </button>

  {#if !capsule}
    <!-- Disabled rather than absent: a creator should be able to see that the feature
         exists and that this server is why it is not available. -->
    <button
      class="rad-self__btn rad-self__btn--record"
      type="button"
      disabled={!state.recordAllowed}
      aria-pressed={state.recording}
      aria-label={state.recording ? "Stop recording" : "Start recording"}
      title={state.recordAllowed ? undefined : recordBlockedLabel}
      onclick={onrecord}
    >
      <!-- The glyph pulses while recording; a separate blinking dot would state the
           same fact twice. -->
      <Icon name="rec" />
      {#if state.recordAllowed}
        <span class="rad-self__rec-time">{recordTime}</span>
      {/if}
    </button>
  {/if}
</div>
