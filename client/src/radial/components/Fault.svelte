<script lang="ts">
  import Icon from "./Icon.svelte";
  import Ring from "./Ring.svelte";
  import type { Severity } from "$radial/core/controllers/Diagnostics";
  import type { IconName } from "$radial/core/icons/Icons";

  interface Props {
    /** Which part of the app this is about. */
    icon: IconName;
    /** bad · something is broken. warn · someone has to act. ok · not a failure at all. */
    severity?: Severity;
  }

  let { icon, severity = "bad" }: Props = $props();

  /**
   * Lower right, and the same on every screen. Far enough from the caption in the bottom
   * left to leave it alone, and open toward the copy beside it. Deriving the angle from
   * the error code was considered and dropped: a break that lands somewhere different
   * each time is a private joke, not information.
   */
  const CUT = [-Math.PI / 2 + 2.28, 0.46] as const;

  /**
   * Token values, not tokens. The canvas parses a colour it can read channels out of, and
   * `var(--color-rad-fault)` is not one — the same reason `RingBinding` carries the empty
   * ring's violets as literals. `--color-rad-fault` and `--color-rad-warn`.
   */
  const CUT_TONE: Partial<Record<Severity, string>> = { bad: "#ff8266", warn: "#ffcf4d" };
</script>

<!--
  A circle that cannot be completed.

  The gap is the whole message, and it does not care what broke: a missing microphone, a
  blocked UDP path and a client too old to connect all leave the same hole. The glyph at
  the centre says which part of the app, subordinate to the break rather than announcing
  it, and the mark gives up the middle to it — the logo is in the top bar on every screen
  that shows this.

  `ok` is the exception and the only complete ring in the set: whole, turning, painted from
  the mark's own spectrum. An available update is good news, and news is not a fault with a
  different colour.
-->
<div class="rad-fault {severity !== 'bad' ? `rad-fault--${severity}` : ''}">
  {#if severity === "ok"}
    <Ring mode="live" spectrum spin={0.16} mark={false} class="rad-ring--fill" />
  {:else}
    <Ring
      mode="empty"
      cut={CUT}
      cutTone={CUT_TONE[severity]}
      mark={false}
      ringStill
      class="rad-ring--fill"
    />
  {/if}
  <span class="rad-fault__glyph"><Icon name={icon} /></span>
</div>
