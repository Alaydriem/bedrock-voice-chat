<script lang="ts">
  import type { ChatMessage } from "$radial/core/controllers/ChatLog";

  interface Props {
    message: ChatMessage;
    /** Their identity colour. Falls back to neutral for anyone not in the roster. */
    hue?: string;
  }

  let { message, hue = "#9483b6" }: Props = $props();

  const initial = $derived((message.author ?? "").slice(0, 1).toUpperCase());
</script>

<!-- Text is interpolated, never `@html`. Chat is the most obviously attacker-controlled
     string in the product: it arrives from other players over the network. -->
{#if message.system}
  <div class="rad-msg rad-msg--system">
    <span class="rad-msg__avatar">·</span>
    <span class="rad-msg__text">{message.text}</span>
    <span class="rad-msg__ts">{message.timestamp}</span>
  </div>
{:else}
  <div class="rad-msg" class:rad-msg--mention={message.mention}>
    <span class="rad-msg__avatar" style="background:{hue}">{initial}</span>
    <span class="rad-msg__text">
      <span class="rad-msg__author" style="color:{hue}">{message.author}</span>{message.text}{#if message.fromApp}<span
          class="rad-msg__app"
          title="sent from the app, not in game"
        ></span>{/if}
    </span>
    <span class="rad-msg__ts">{message.timestamp}</span>
  </div>
{/if}
