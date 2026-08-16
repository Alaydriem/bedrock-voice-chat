<script lang="ts">
    import { I18n } from "$lib/i18n";
    import Icon from "$radial/components/Icon.svelte";
    import type { ChatLine } from "../../js/app/chat/ChatLine";
    import type { ChatRejectionState, ChatTarget } from "../../js/app/chat/ChatTarget";
    import ChatComposer from "./ChatComposer.svelte";
    import type { WorldAssociations } from "../../js/app/chat/WorldLabel";
    import ChatMessageRow from "./ChatMessageRow.svelte";
    import ChatTargetPicker from "./ChatTargetPicker.svelte";
    import type { ChatWorld } from "../../js/bindings/ChatWorld";

    interface Props {
        lines: ChatLine[];
        target: ChatTarget;
        rejection: ChatRejectionState | null;
        unread: number;
        open: boolean;
        /** Remembered world names, keyed by uuid, for the labels the picker renders. */
        associations?: WorldAssociations;
        /** Falls back to a neutral for anyone not in the roster. */
        hueOf: (author: string) => string;
        onToggle: (open: boolean) => void;
        onSend: (text: string) => void;
        onDismissRejection?: () => void;
        onPickWorld?: (world: ChatWorld) => void;
    }
    let {
        lines,
        target,
        rejection,
        unread,
        open,
        associations = {},
        hueOf,
        onToggle,
        onSend,
        onDismissRejection,
        onPickWorld,
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

    // Only offered when the player is out of game with more than one world available.
    let canChoose = $derived(target.kind === "choose");
    let pickerOpen = $state(false);

    let label = $derived(
        target.kind === "local" || target.kind === "unavailable"
            ? I18n.t("Server chat")
            : target.world.world_name,
    );

    let status = $derived(
        target.kind === "unavailable"
            ? { text: I18n.t("Off"), cls: "rad-status-chip--warn" }
            : target.kind === "in-game"
              ? { text: I18n.t("In game"), cls: "rad-status-chip--live" }
              : { text: I18n.t("Live"), cls: "rad-status-chip--live" },
    );
</script>

<div class="rad-chat-dock">
    <div class="rad-chat-history">
        <div class="rad-chat__head">
            <!-- A button only when the choice is real. In game, or with one world, there is
                 nothing to pick and the caret is hidden. -->
            <button
                class="rad-chat-target"
                class:is-static={!canChoose}
                onclick={() => canChoose && (pickerOpen = true)}
                aria-label={I18n.t("Where this message goes")}
            >
                <span class="rad-chat-target__name">{label}</span>
                <span class="rad-chat-target__caret"><Icon name="chev" /></span>
            </button>
            <span class="rad-status-chip {status.cls}">{status.text}</span>
            <span class="rad-spacer"></span>
            <button class="rad-icon-btn" onclick={() => onToggle(false)} aria-label={I18n.t("Close chat")}>
                <Icon name="close" />
            </button>
        </div>

        <div class="rad-chat__body" bind:this={body} onscroll={trackScroll}>
            <!--
              Said in the scrollback rather than in a privacy notice, because the person
              deciding what to type needs to know it then.
            -->
            <div class="rad-chat__note">
                {I18n.t("History starts when you connect — nothing is stored")}
            </div>
            {#each lines as line, i (i)}
                <ChatMessageRow {line} hue={hueOf(line.author ?? "")} />
            {/each}
        </div>
    </div>

    <ChatComposer
        {target}
        {rejection}
        {unread}
        {associations}
        {onSend}
        onToggle={() => onToggle(!open)}
        onFocus={() => onToggle(true)}
        {onDismissRejection}
    />
</div>

{#if pickerOpen && target.kind === "choose"}
    <!-- A button rather than a div: dismissing has to work from a keyboard too. -->
    <button
        class="rad-scrim"
        onclick={() => (pickerOpen = false)}
        aria-label="Close world picker"
    ></button>
    <ChatTargetPicker
        options={target.options}
        current={target.world}
        {associations}
        onPick={(world) => {
            onPickWorld?.(world);
            pickerOpen = false;
        }}
        onClose={() => (pickerOpen = false)}
    />
{/if}
