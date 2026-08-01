/**
 * Whether an element is worth painting this frame.
 *
 * The kit puts several canvases behind `hidden` panes, closed screens and
 * collapsed sheets at once. Painting them costs the same as painting the visible
 * one and shows nobody anything, so every renderer asks first.
 */
export class Visibility {
  static isPaintable(el: HTMLElement): boolean {
    // offsetParent is null for display:none and for any display:none ancestor,
    // which covers [hidden], a closed screen, and a collapsed pane in one check.
    // position:fixed elements report null too, so they are allowed through.
    if (el.offsetParent === null && getComputedStyle(el).position !== "fixed") return false;
    return el.offsetWidth > 0 && el.offsetHeight > 0;
  }

  static prefersReducedMotion(): boolean {
    return typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches;
  }
}
