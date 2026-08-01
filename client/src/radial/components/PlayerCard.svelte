<script lang="ts">
  import LevelMeter from "./LevelMeter.svelte";
  import Icon from "./Icon.svelte";
  import type { LevelSource } from "$radial/core/sources/LevelSource";

  interface Props {
    name: string;
    /** Their identity colour. PlayerHue.of("game:gamertag") derives it. */
    hue: string;
    /** Metres away, or null when they are in a channel and distance does not apply. */
    distance?: number | null;
    source?: LevelSource;
    /** Per-player volume, 0 to 1.5. */
    gain?: number;
    muted?: boolean;
    /** Held back until the handoff flyer lands. */
    pending?: boolean;
    onmute?: (muted: boolean) => void;
    ongain?: (gain: number) => void;
  }

  let {
    name,
    hue,
    distance = null,
    source,
    gain = $bindable(1),
    muted = $bindable(false),
    pending = false,
    onmute,
    ongain,
  }: Props = $props();

  let live = $state(false);

  const initials = $derived(name.slice(0, 2).toUpperCase());
  const range = $derived(distance === null ? "IN GROUP" : `${Math.round(distance)} M`);

  function toggleMute() {
    muted = !muted;
    onmute?.(muted);
  }

  function setGain(e: Event) {
    gain = Number((e.currentTarget as HTMLInputElement).value);
    ongain?.(gain);
  }
</script>

<!-- The card sets `color` to their hue, so the avatar ring, the slider thumb and the
     meter all take it through currentColor without being told whose card they are in. -->
<div
  class="rad-player"
  class:is-pending={pending}
  class:is-live={live && !muted}
  style="color:{hue}"
  data-rad-live-target
>
  <div class="rad-player__avatar" style="background:{hue}">
    <span class="rad-player__ring"></span>{initials}
  </div>
  <div>
    <div class="rad-player__head">
      <span class="rad-player__name">{name}</span>
      <span class="rad-player__distance">{range}</span>
    </div>
    <div class="rad-player__level">
      <LevelMeter {source} color={muted ? "#7a68a0" : hue} onlive={(v) => (live = v)} />
    </div>
    <div class="rad-player__controls">
      <button
        class="rad-player__mute"
        type="button"
        aria-pressed={muted}
        aria-label="{muted ? 'Unmute' : 'Mute'} {name}"
        onclick={toggleMute}
      >
        <Icon name={muted ? "micoff" : "mic"} />
      </button>
      <input
        class="rad-range"
        type="range"
        min="0"
        max="1.5"
        step="0.05"
        value={gain}
        disabled={muted}
        aria-label="Volume for {name}"
        oninput={setGain}
      />
      <span class="rad-player__percent">{Math.round(gain * 100)}%</span>
    </div>
  </div>
</div>
