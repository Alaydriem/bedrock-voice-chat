<script lang="ts">
    import { onDestroy, onMount, type Snippet } from "svelte";
    import { CoverDrag } from "$radial/core/controllers/CoverDrag";

    interface Props {
        open: boolean;
        /** The screen this one is presented over. Stays mounted, and stays visible. */
        under?: Snippet;
        children?: Snippet;
        /** Escape, the scrim, or the platform back. An explicit close calls its own handler. */
        ondismiss: () => void;
    }
    let { open, under, children, ondismiss }: Props = $props();

    let host = $state<HTMLElement | null>(null);

    function onkeydown(e: KeyboardEvent): void {
        if (e.key !== "Escape" || !open) return;
        // A menu or a modal on top owns Escape first.
        if (host?.closest(".rad-frame")?.querySelector(".rad-menu.is-open, .rad-modal.is-open")) {
            return;
        }
        e.preventDefault();
        ondismiss();
    }

    onMount(() => document.addEventListener("keydown", onkeydown));
    onDestroy(() => document.removeEventListener("keydown", onkeydown));

    /* ---- drag down to dismiss ----
     * `CoverDrag` holds the arithmetic; this holds the pointer events.
     */

    let cover = $state<HTMLElement | null>(null);
    let startY = 0;
    let dragging = $state(false);
    let offset = $state(0);

    /** True only when the content under the finger has nothing left to scroll. */
    function claims(target: EventTarget | null): boolean {
        const scroller = (target as HTMLElement | null)?.closest<HTMLElement>(
            ".rad-settings-body, .rad-mobile-list, .rad-modal__scroll",
        );
        return CoverDrag.canStart(scroller?.scrollTop ?? 0);
    }

    function onpointerdown(e: PointerEvent): void {
        // A modal on top owns its own gestures, and a drag on a slider is not a dismiss.
        if ((e.target as HTMLElement).closest(".rad-modal, .rad-range, input, select")) return;
        if (!open || !claims(e.target)) return;
        startY = e.clientY;
        offset = 0;
        dragging = true;
        try {
            cover?.setPointerCapture(e.pointerId);
        } catch {
            // Synthetic events have no live pointer.
        }
    }

    function onpointermove(e: PointerEvent): void {
        if (!dragging) return;
        const dy = e.clientY - startY;
        if (!CoverDrag.isDrag(dy) && offset === 0) return;
        offset = CoverDrag.offset(dy);
    }

    function onpointerup(e: PointerEvent): void {
        if (!dragging) return;
        const travelled = offset;
        dragging = false;
        offset = 0;
        try {
            cover?.releasePointerCapture(e.pointerId);
        } catch {
            // Never captured, so nothing to release.
        }
        if (CoverDrag.dismisses(travelled)) ondismiss();
    }
</script>

<!-- One screen over another. The screen behind stays mounted. -->
<div bind:this={host} style="display: contents">
    <!-- `inert` keeps the covered screen out of the tab order. -->
    <div class="rad-under" class:is-covered={open} inert={open}>
        {@render under?.()}
    </div>

    <div
        class="rad-scrim rad-scrim--cover"
        class:is-on={open}
        onclick={() => open && ondismiss()}
        role="presentation"
    ></div>

    <!-- A supplementary touch gesture. Escape, the back button and the scrim all remain. -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
        bind:this={cover}
        class="rad-cover"
        class:is-open={open}
        class:is-dragging={dragging}
        style={offset > 0 ? `transform: translateY(${offset}px)` : ""}
        {onpointerdown}
        {onpointermove}
        {onpointerup}
        onpointercancel={onpointerup}
    >
        <!-- The grip is the hit area; the bar is what is seen. Decorative. -->
        <span class="rad-cover__grip" aria-hidden="true">
            <span class="rad-cover__handle"></span>
        </span>
        {@render children?.()}
    </div>
</div>
