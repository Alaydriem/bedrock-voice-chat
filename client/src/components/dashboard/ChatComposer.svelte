<script lang="ts">
    import { I18n } from "$lib/i18n";
    import Icon from "$radial/components/Icon.svelte";
    import type { ChatRejectionState, ChatTarget } from "../../js/app/chat/ChatTarget";
    import { WorldLabel, type WorldAssociations } from "../../js/app/chat/WorldLabel";

    interface Props {
        target: ChatTarget;
        rejection: ChatRejectionState | null;
        unread: number;
        /** Remembered world names, keyed by uuid. Empty until one has been learned. */
        associations?: WorldAssociations;
        onSend: (text: string) => void;
        onToggle: () => void;
        onFocus?: () => void;
        onDismissRejection?: () => void;
    }
    let {
        target,
        rejection,
        unread,
        associations = {},
        onSend,
        onToggle,
        onFocus,
        onDismissRejection,
    }: Props =
        $props();

    let text = $state("");
    let unavailable = $derived(target.kind === "unavailable");
    let disabled = $derived(target.kind === "disabled");

    // The target is named where the typing happens, not only in the header. The scrollback is
    // shut most of the time, and the moment that matters is the one where somebody is deciding
    // what to send.
    let placeholder = $derived(
        target.kind === "disabled" || target.kind === "unavailable"
            ? target.reason
            : target.kind === "local"
              ? I18n.t("Message the server…")
              : I18n.tf("Message {world}…", {
                    world: WorldLabel.resolve(
                        target.world.world_uuid,
                        target.world.world_name,
                        associations,
                    ),
                }),
    );

    // Typing is never refused while a target may yet appear. A line nothing can carry is
    // rendered unconfirmed and the sender can read it back; swallowing it, as a disabled
    // composer did, taught them nothing. A server that turned chat off is the exception —
    // there is no later moment when the line lands.
    function submit(): void {
        if (disabled) return;
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

<div
    class="rad-chat-bar"
    class:is-unavailable={unavailable || disabled}
    class:is-disabled={disabled}
>
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
        {disabled}
        autocomplete="off"
        aria-label={placeholder}
        onfocus={onFocus}
        onkeydown={(e) => e.key === "Enter" && submit()}
    />
    <button
        class="rad-chat-send"
        class:is-ready={!disabled && text.trim().length > 0}
        {disabled}
        onclick={submit}
        aria-label={I18n.t("Send")}
    >
        <Icon name="send" />
    </button>
</div>
