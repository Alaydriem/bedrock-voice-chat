<script lang="ts">
  import type { Snippet } from "svelte";
  import Fault from "$radial/components/Fault.svelte";
  import Loader from "$radial/components/Loader.svelte";
  import type { Severity } from "$radial/core/controllers/Diagnostics";
  import type { IconName } from "$radial/core/icons/Icons";
  import RadScreen from "../shell/RadScreen.svelte";

  interface Action {
    label: string;
    onclick: () => void;
    primary?: boolean;
    disabled?: boolean;
  }

  interface Props {
    /** The reference, in mono. Not the headline — the title does that job. */
    code: string;
    title: string;
    message: string;
    icon: IconName;
    /** bad · broken. warn · someone has to act. ok · the update, which is good news. */
    severity?: Severity;
    /** What this is about: the caption's label, and the eyebrow over the title. */
    category: string;
    /**
     * Replaces the eyebrow with a severity chip. For a screen that is not reporting a
     * break, where the first thing to say is that nothing is wrong.
     */
    chip?: string;
    /** The state in two or three words, beside the code. */
    caption: string;
    /** Right of the top bar. */
    label: string;
    /** Left of the footbar: the usual cause, or a reassurance. */
    hint: string;
    appVersion: string;
    actions: readonly Action[];
    /**
     * Replaces the visual with the mark spinner. The update is the longest wait in the
     * app, and a borrowed spinner said only that something was happening.
     */
    working?: boolean;
    workingPhrases?: readonly string[];
    /** Links under the actions, when there is somewhere useful to send someone. */
    footnote?: Snippet;
  }

  let {
    code,
    title,
    message,
    icon,
    severity = "bad",
    category,
    chip,
    caption,
    label,
    hint,
    appVersion,
    actions,
    working = false,
    workingPhrases,
    footnote,
  }: Props = $props();

  const CHIP: Record<Severity, string> = {
    bad: "rad-status-chip--bad",
    warn: "rad-status-chip--warn",
    ok: "rad-status-chip--ok",
  };
</script>

<!--
  Every terminal state in the app: the fifteen error codes, and the update, which is on
  this route because it is also a screen you cannot get past without deciding something.

  The code is set in mono in the caption and the footbar rather than at display size. It
  is a reference someone pastes into a support thread, and it was never the thing they
  needed to read first.
-->
<RadScreen {label}>
  <div class="rad-split">
    <div class="rad-visual-pane">
      <!-- Same placement as the login flow's connecting screen: the loader takes the pane
           whole, without the caption, because what it is doing is written under the mark. -->
      {#if working}
        <Loader loading={true} phrases={workingPhrases} slowAfterSeconds={4} />
      {:else}
        <div class="rad-visual">
          <Fault {icon} {severity} />
          <span class="rad-caption">
            <span class="rad-label">{category}</span>
            <span class="rad-caption__value">{caption} · {code}</span>
          </span>
        </div>
      {/if}
    </div>

    <div class="rad-content-pane">
      {#if chip}
        <span
          class="rad-status-chip {CHIP[severity]} rad-rise"
          style="--d: 50; align-self: flex-start"
        >
          {chip}
        </span>
      {:else}
        <span class="rad-label rad-rise" style="--d: 50">{category}</span>
      {/if}
      <h2 class="rad-display rad-rise" style="--d: 120; margin-top: 12px; font-size: 2rem">
        {title}
      </h2>
      <p class="rad-body rad-rise" style="--d: 210">{message}</p>

      <div class="rad-rise" style="--d: 300; margin-top: 22px; max-width: 370px">
        {#each actions as action, i (action.label)}
          <button
            class="rad-btn rad-btn--lg {action.primary ? 'rad-btn--primary' : ''}"
            style="width: 100%{i > 0 ? '; margin-top: 10px' : ''}"
            disabled={action.disabled === true}
            onclick={action.onclick}
          >
            {action.label}
          </button>
        {/each}
      </div>

      {#if footnote}
        {@render footnote()}
      {/if}
    </div>
  </div>

  {#snippet footbar()}
    <span class="rad-label">{hint}</span>
    <span class="rad-label rad-num">{code} · v{appVersion}</span>
  {/snippet}
</RadScreen>
