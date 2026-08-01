<script lang="ts">
  import type { Severity } from "$radial/core/controllers/Diagnostics";

  interface Props {
    severity?: Severity;
    text: string;
  }

  let { severity = "ok", text }: Props = $props();
</script>

<!-- The headline of the status panel: the first failing check, in a sentence, in plain
     language. Someone who opened this panel has a problem, and leading with numbers
     makes them do the diagnosis themselves. Diagnostics.verdict() decides which one. -->
<div class="rad-verdict {severity !== 'ok' ? `rad-verdict--${severity}` : ''}" role="status">
  <span class="rad-verdict__dot"></span>
  <span class="rad-verdict__text">{text}</span>
</div>
