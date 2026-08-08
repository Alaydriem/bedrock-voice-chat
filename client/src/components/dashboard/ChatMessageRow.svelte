<script lang="ts">
    import type { ChatLine } from "../../js/app/chat/ChatLine";

    interface Props {
        line: ChatLine;
        /** Falls back to a neutral for anyone not in the roster. */
        hue: string;
    }
    let { line, hue }: Props = $props();

    let initial = $derived((line.author ?? "").slice(0, 1).toUpperCase());
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
    <div class="rad-msg" class:rad-msg--mention={line.mention}>
        <span class="rad-msg__avatar" style="background:{hue}">{initial}</span>
        <span class="rad-msg__text">
            <span class="rad-msg__author" style="color:{hue}">{line.author}</span>{line.text}{#if line.fromApp}<span
                    class="rad-msg__app"
                    title="sent from the app, not in game"
                ></span>{/if}
        </span>
        <span class="rad-msg__ts">{line.timestamp}</span>
    </div>
{/if}
