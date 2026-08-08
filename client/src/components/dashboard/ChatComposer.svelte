<script lang="ts">
    import Icon from "$radial/components/Icon.svelte";
    import type { ChatRejectionState, ChatTarget } from "../../js/app/chat/ChatTarget";

    interface Props {
        target: ChatTarget;
        rejection: ChatRejectionState | null;
        unread: number;
        onSend: (text: string) => void;
        onToggle: () => void;
        onFocus?: () => void;
        onDismissRejection?: () => void;
    }
    let { target, rejection, unread, onSend, onToggle, onFocus, onDismissRejection }: Props =
        $props();

    let text = $state("");
    let unavailable = $derived(target.kind === "unavailable");

    // The target is named where the typing happens, not only in the header. The scrollback is
    // shut most of the time, and the moment that matters is the one where somebody is deciding
    // what to send.
    let placeholder = $derived(
        target.kind === "unavailable"
            ? target.reason
            : target.kind === "local"
              ? "Message the server…"
              : `Message ${target.world.world_name}…`,
    );

    function submit(): void {
        const trimmed = text.trim();
        if (!trimmed || unavailable) return;
        onSend(trimmed);
        text = "";
    }
</script>

{#if rejection}
    <button class="rad-chat-notice" onclick={() => onDismissRejection?.()}>
        {#if rejection.kind === "moved"}
            You moved out of {rejection.from} — that message was not sent
        {:else}
            {rejection.reason} — that message was not sent
        {/if}
    </button>
{/if}

<div class="rad-chat-bar" class:is-unavailable={unavailable}>
    <button class="rad-chat-toggle" onclick={onToggle} aria-label="Chat">
        <Icon name="chat" />
        {#if unread > 0}
            <span class="rad-chat-badge">{unread > 9 ? "9+" : unread}</span>
        {/if}
    </button>
    <input
        class="rad-chat-input"
        bind:value={text}
        {placeholder}
        disabled={unavailable}
        autocomplete="off"
        aria-label={placeholder}
        onfocus={onFocus}
        onkeydown={(e) => e.key === "Enter" && submit()}
    />
    <button
        class="rad-chat-send"
        class:is-ready={text.trim().length > 0}
        onclick={submit}
        aria-label="Send"
    >
        <Icon name="send" />
    </button>
</div>
