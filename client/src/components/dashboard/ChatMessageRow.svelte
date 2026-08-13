<script lang="ts">
    import { I18n } from "$lib/i18n";
    import type { ChatLine } from "../../js/app/chat/ChatLine";

    interface Props {
        line: ChatLine;
        /** Falls back to a neutral for anyone not in the roster. */
        hue: string;
    }
    let { line, hue }: Props = $props();

    let initial = $derived((line.author ?? "").slice(0, 1).toUpperCase());

    // An unconfirmed line drops its identity colour rather than only fading it. Fading a hue
    // leaves a recognisably coloured avatar and name, which still reads as a delivered line.
    let tone = $derived(line.delivery === "confirmed" ? hue : "var(--color-rad-dim)");
</script>

<!--
  Every author and every message body renders through Svelte interpolation, never {@html}.
  Chat text arrives from other players and is the most attacker-controlled string in the
  product; the radial prototype interpolated it straight into innerHTML.
-->
{#if line.system}
    <div class="rad-msg rad-msg--system">
        <span class="rad-msg__avatar">·</span>
        <span class="rad-msg__text">{line.text}</span>
        <span class="rad-msg__ts">{line.timestamp}</span>
    </div>
{:else}
    <!-- Unconfirmed reads quietly rather than being hidden: the sender has to be able to
         read back what they typed even when nothing has proven it arrived. -->
    <div
        class="rad-msg"
        class:rad-msg--mention={line.mention}
        class:rad-msg--unconfirmed={line.delivery !== "confirmed"}
        title={line.delivery === "pending"
            ? I18n.t("sending…")
            : line.delivery === "failed"
              ? I18n.t("not confirmed — this may not have been delivered")
              : undefined}
    >
        <span class="rad-msg__avatar" style="background:{tone}">{initial}</span>
        <span class="rad-msg__text">
            <span class="rad-msg__author" style="color:{tone}">{line.author}</span>{line.text}{#if line.fromApp}<span
                    class="rad-msg__app"
                    title={I18n.t("sent from the app, not in game")}
                ></span>{/if}
        </span>
        <span class="rad-msg__ts">{line.timestamp}</span>
    </div>
{/if}
