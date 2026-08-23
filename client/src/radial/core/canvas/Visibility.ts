/**
 * Whether an element is worth painting this frame.
 *
 * The kit puts several canvases behind `hidden` panes, closed screens and
 * collapsed sheets at once. Painting them costs the same as painting the visible
 * one and shows nobody anything, so every renderer asks first.
 */
export class Visibility {
  /**
   * Elements currently scrolled out of view.
   *
   * `offsetParent` catches a hidden pane but not a scrolled one, so a roster of forty
   * players used to paint forty canvases every frame with six of them on screen. One shared
   * observer answers for every renderer at once, and registration happens on the first
   * `isPaintable` call, so no call site has to know this exists.
   *
   * A weak set, because a canvas that is removed from the document must not be kept alive by
   * the thing that decided whether to draw it.
   */
  static #offscreen = new WeakSet<Element>();
  static #watched = new WeakSet<Element>();
  static #observer: IntersectionObserver | null = null;

  /**
   * Elements the observer has affirmatively reported as visible at least once.
   *
   * Fail-open, and the reason is a deadlock: every renderer asks this before sizing its canvas,
   * so an element reported non-intersecting while it still has no layout would never paint,
   * never gain a size, and never move — nothing left to trigger a second callback. Trusting a
   * negative only after a positive means the worst case is painting something invisible, which
   * is what the whole kit did before this existed.
   */
  static #confirmed = new WeakSet<Element>();

  static isPaintable(el: HTMLElement): boolean {
    Visibility.#watch(el);
    if (Visibility.#confirmed.has(el) && Visibility.#offscreen.has(el)) return false;
    // offsetParent is null for display:none and for any display:none ancestor,
    // which covers [hidden], a closed screen, and a collapsed pane in one check.
    // position:fixed elements report null too, so they are allowed through.
    if (el.offsetParent === null && getComputedStyle(el).position !== "fixed") return false;
    return el.offsetWidth > 0 && el.offsetHeight > 0;
  }

  static #watch(el: HTMLElement): void {
    if (Visibility.#watched.has(el)) return;
    // Absent under a test DOM, and its absence must not blank every canvas: without an
    // observer nothing is ever marked offscreen, which is the behaviour this replaced.
    if (typeof IntersectionObserver === "undefined") return;

    Visibility.#watched.add(el);
    Visibility.#observer ??= new IntersectionObserver((entries) => {
      for (const entry of entries) {
        if (entry.isIntersecting) {
          Visibility.#confirmed.add(entry.target);
          Visibility.#offscreen.delete(entry.target);
        } else {
          Visibility.#offscreen.add(entry.target);
        }
      }
    });
    Visibility.#observer.observe(el);
  }

  static prefersReducedMotion(): boolean {
    return typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches;
  }
}
