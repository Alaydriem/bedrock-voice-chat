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
    /** A press that is allowed to become a drag, before it has become one. */
    let pressed = false;
    /** The pointer this cover holds, or null while it holds none. */
    let held: number | null = null;
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
        // A press on a control is that control's. A modal on top owns its own gestures, and a
        // drag on a slider is not a dismiss. Same list the kit's sheet uses.
        if ((e.target as HTMLElement).closest(".rad-modal, .rad-range, button, a, input, select")) {
            return;
        }
        if (!open || !claims(e.target)) return;
        startY = e.clientY;
        offset = 0;
        pressed = true;
    }

    function onpointermove(e: PointerEvent): void {
        if (!pressed) return;
        const dy = e.clientY - startY;
        if (!CoverDrag.isDrag(dy) && offset === 0) return;
        // The pointer is taken once the gesture is a drag, and never on the press itself: the
        // browser sends the click to whichever element holds the pointer, so capturing early
        // makes the cover swallow the click and leaves everything inside it inert.
        if (!dragging) {
            dragging = true;
            try {
                cover?.setPointerCapture(e.pointerId);
                held = e.pointerId;
            } catch {
                // Synthetic events have no live pointer.
            }
        }
        offset = CoverDrag.offset(dy);
    }

    function onpointerup(): void {
        if (!pressed) return;
        const travelled = offset;
        pressed = false;
        dragging = false;
        offset = 0;
        if (held !== null) {
            try {
                cover?.releasePointerCapture(held);
            } catch {
                // The pointer is already gone.
            }
            held = null;
        }
        if (CoverDrag.dismisses(travelled)) ondismiss();
    }

    /**
     * The browser reclaiming the pointer mid-drag — the webview backgrounded, the gesture
     * handed to the system. No pointerup is coming, so the drag ends here: spring back
     * rather than dismiss, because the user never finished asking to leave. Also fires
     * after our own release in `onpointerup`, where everything is already reset.
     *
     * Only the cover's own loss counts. On touch, pointerdown implicitly captures the
     * pointer to the element under the finger, and taking it for the cover makes that
     * child announce the transfer with a lostpointercapture that bubbles up here — the
     * start of every touch drag, not the end of one.
     */
    function onlostpointercapture(e: PointerEvent): void {
        if (e.target !== cover) return;
        pressed = false;
        dragging = false;
        offset = 0;
        held = null;
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
        {onlostpointercapture}
    >
        <!-- The grip is the hit area; the bar is what is seen. Decorative. -->
        <span class="rad-cover__grip" aria-hidden="true">
            <span class="rad-cover__handle"></span>
        </span>
        {@render children?.()}
    </div>
</div>
