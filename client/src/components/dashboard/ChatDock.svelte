<script lang="ts">
    import Icon from "$radial/components/Icon.svelte";
    import type { ChatLine } from "../../js/app/chat/ChatLine";
    import type { ChatRejectionState, ChatTarget } from "../../js/app/chat/ChatTarget";
    import ChatComposer from "./ChatComposer.svelte";
    import ChatMessageRow from "./ChatMessageRow.svelte";

    interface Props {
        lines: ChatLine[];
        target: ChatTarget;
        rejection: ChatRejectionState | null;
        unread: number;
        open: boolean;
        /** Falls back to a neutral for anyone not in the roster. */
        hueOf: (author: string) => string;
        onToggle: (open: boolean) => void;
        onSend: (text: string) => void;
        onDismissRejection?: () => void;
    }
    let {
        lines,
        target,
        rejection,
        unread,
        open,
        hueOf,
        onToggle,
        onSend,
        onDismissRejection,
    }: Props = $props();

    // Autoscroll only when the reader was already at the bottom. Yanking somebody away from a
    // line they were reading because a stranger said "lol" is worse than making them scroll.
    const STICK_PX = 40;

    let body = $state<HTMLElement | null>(null);
    let atEnd = $state(true);

    function trackScroll(): void {
        if (!body) return;
        atEnd = body.scrollTop + body.clientHeight >= body.scrollHeight - STICK_PX;
    }

    $effect(() => {
        // Read the length so this re-runs as chat arrives.
        lines.length;
        if (body && atEnd) {
            body.scrollTop = body.scrollHeight;
        }
    });

    let label = $derived(
        target.kind === "local" || target.kind === "unavailable"
            ? "Server chat"
            : target.world.world_name,
    );

    let status = $derived(
        target.kind === "unavailable"
            ? { text: "Off", cls: "rad-status-chip--warn" }
            : target.kind === "in-game"
              ? { text: "In game", cls: "rad-status-chip--live" }
              : { text: "Live", cls: "rad-status-chip--live" },
    );
</script>

<div class="rad-chat-dock">
    <div class="rad-chat-history">
        <div class="rad-chat__head">
            <!-- Static until a world picker exists: with one world there is nothing to pick. -->
            <span class="rad-chat-target is-static">
                <span class="rad-chat-target__name">{label}</span>
            </span>
            <span class="rad-status-chip {status.cls}">{status.text}</span>
            <span class="rad-spacer"></span>
            <button class="rad-icon-btn" onclick={() => onToggle(false)} aria-label="Close chat">
                <Icon name="close" />
            </button>
        </div>

        <div class="rad-chat__body" bind:this={body} onscroll={trackScroll}>
            <!--
              Said in the scrollback rather than in a privacy notice, because the person
              deciding what to type needs to know it then.
            -->
            <div class="rad-chat__note">History starts when you connect — nothing is stored</div>
            {#each lines as line, i (i)}
                <ChatMessageRow {line} hue={hueOf(line.author ?? "")} />
            {/each}
        </div>
    </div>

    <ChatComposer
        {target}
        {rejection}
        {unread}
        {onSend}
        onToggle={() => onToggle(!open)}
        onFocus={() => onToggle(true)}
        {onDismissRejection}
    />
</div>
