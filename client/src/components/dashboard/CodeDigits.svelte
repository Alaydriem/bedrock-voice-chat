<script lang="ts">
    import { I18n } from "#lib/i18n/index.js";

    interface Props {
        /** "dddd dddd", as both apps show it. */
        code: string;
    }
    let { code }: Props = $props();
</script>

<!-- The most prominent thing on the card or the dialog: people compare it digit by digit. -->
<div class="rad-digits" aria-label={I18n.tf("Code {code}", { code })}>{code}</div>
