<script lang="ts">
    import type { Snippet } from "svelte";

    interface Props {
        open: boolean;
        title: string;
        /** The consequence, spelled out. The title asks; this says what happens. */
        body: Snippet;
        confirmLabel: string;
        cancelLabel?: string;
        /** Whether confirming destroys something, which sets the button's colour. */
        destructive?: boolean;
        onconfirm: () => void;
        oncancel: () => void;
    }
    let {
        open,
        title,
        body,
        confirmLabel,
        cancelLabel = "Cancel",
        destructive = false,
        onconfirm,
        oncancel,
    }: Props = $props();

    /**
     * Escape cancels. The scrim is a click target too, and both resolve to the same
     * answer: dismissing a confirm is never the destructive branch.
     */
    function onkeydown(event: KeyboardEvent): void {
        if (open && event.key === "Escape") oncancel();
    }
</script>

<svelte:window {onkeydown} />

<!--
  Positioned against `.rad-app-stage` like every other overlay in the kit, so it covers
  the frame rather than the document — which still carries the old theme's body classes.

  The scrim keeps its `is-on` class rather than being removed from the DOM: the kit
  animates both in and out, and an element that vanishes cannot fade.
-->
<div
    class="rad-scrim rad-scrim--modal {open ? 'is-on' : ''}"
    onclick={oncancel}
    aria-hidden="true"
></div>

<div
    class="rad-modal {open ? 'is-open' : ''}"
    role="dialog"
    aria-modal="true"
    aria-label={title}
    aria-hidden={!open}
>
    <h5 class="rad-modal__title">{title}</h5>
    <p>{@render body()}</p>
    <div class="rad-modal__actions">
        <button class="rad-btn" onclick={oncancel} tabindex={open ? 0 : -1}>{cancelLabel}</button>
        <button
            class="rad-btn {destructive ? 'rad-btn--danger' : 'rad-btn--primary'}"
            onclick={onconfirm}
            tabindex={open ? 0 : -1}
        >
            {confirmLabel}
        </button>
    </div>
</div>
