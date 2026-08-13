<script lang="ts">
    import { I18n } from "$lib/i18n";
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
              ? I18n.t("Message the server…")
              : I18n.tf("Message {world}…", { world: target.world.world_name }),
    );

    // Typing is never refused. A line nothing can carry is rendered unconfirmed and the sender
    // can read it back; swallowing it, as a disabled composer did, taught them nothing.
    function submit(): void {
        const trimmed = text.trim();
        if (!trimmed) return;
        onSend(trimmed);
        text = "";
    }
</script>

{#if rejection}
    <button class="rad-chat-notice" onclick={() => onDismissRejection?.()}>
        {#if rejection.kind === "moved"}
            {I18n.tf("You moved out of {world} — that message was not sent", {
                world: rejection.from,
            })}
        {:else}
            {I18n.tf("{reason} — that message was not sent", { reason: rejection.reason })}
        {/if}
    </button>
{/if}

<div class="rad-chat-bar" class:is-unavailable={unavailable}>
    <button class="rad-chat-toggle" onclick={onToggle} aria-label={I18n.t("Chat")}>
        <Icon name="chat" />
        {#if unread > 0}
            <span class="rad-chat-badge">{unread > 9 ? "9+" : unread}</span>
        {/if}
    </button>
    <input
        class="rad-chat-input"
        bind:value={text}
        {placeholder}
        autocomplete="off"
        aria-label={placeholder}
        onfocus={onFocus}
        onkeydown={(e) => e.key === "Enter" && submit()}
    />
    <button
        class="rad-chat-send"
        class:is-ready={text.trim().length > 0}
        onclick={submit}
        aria-label={I18n.t("Send")}
    >
        <Icon name="send" />
    </button>
</div>
