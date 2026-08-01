import { Visibility } from "../canvas/Visibility";

export interface Point {
  x: number;
  y: number;
}

/**
 * The mark leaving the ring, and coming back.
 *
 * When someone walks into earshot their mark flies out of the ring and lands as their
 * card's avatar; when they leave it flies back. Nobody has to be told that the circle
 * and the list are the same information — they watch it happen once and then they know.
 *
 * Positioned against the viewport because the flight crosses component boundaries, and
 * cleaned up on a timer rather than on `transitionend`, which does not fire if the
 * element is detached mid-flight.
 *
 * Skipped entirely under prefers-reduced-motion: this is a teaching animation, and the
 * thing it teaches is also stated in the layout.
 */
export class Handoff {
  static readonly FLIGHT_MS = 560;
  static readonly BURST_MS = 480;
  static readonly SPARKS = 9;

  /** Centre of an element, in viewport coordinates. */
  static centreOf(el: Element): Point {
    const r = el.getBoundingClientRect();
    return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
  }

  /**
   * Where a player's bar sits on the ring, in viewport coordinates.
   * @param bearing radians, -PI/2 straight up.
   */
  static ringPoint(canvas: HTMLElement, geometry: { cx: number; cy: number; R: number; pitch: number }, bearing: number): Point {
    const r = canvas.getBoundingClientRect();
    const radius = geometry.R + geometry.pitch * 3.2;
    return {
      x: r.left + geometry.cx + Math.cos(bearing) * radius,
      y: r.top + geometry.cy + Math.sin(bearing) * radius,
    };
  }

  /** Fly a block from one point to another. Resolves when it lands. */
  static fly(from: Point, to: Point, hue: string): Promise<void> {
    if (Visibility.prefersReducedMotion()) return Promise.resolve();
    return new Promise((resolve) => {
      const el = document.createElement("div");
      el.className = "rad-flyer";
      el.style.background = hue;
      el.style.left = `${from.x - 7}px`;
      el.style.top = `${from.y - 7}px`;
      el.style.transform = "translate(0,0) scale(.7)";
      document.body.appendChild(el);

      // Two frames: the first commits the start state, the second starts the
      // transition. One frame is not reliably enough for the style to have settled.
      requestAnimationFrame(() =>
        requestAnimationFrame(() => {
          el.style.transform = `translate(${to.x - from.x}px,${to.y - from.y}px) scale(2.6)`;
          el.style.opacity = ".15";
        }),
      );

      setTimeout(() => {
        el.remove();
        resolve();
      }, Handoff.FLIGHT_MS);
    });
  }

  /** A small burst out of a control. Confirms a mute or a deafen landed. */
  static burst(el: Element, hue: string): void {
    Handoff.burstAt(Handoff.centreOf(el), hue);
  }

  /**
   * The same burst from a point captured earlier.
   *
   * Toggling self state re-renders the pill, so by the time a click handler reaches
   * the burst the button it was given no longer exists — and a detached element
   * measures as 0×0 at the origin, putting the sparks in the corner of the screen.
   * Capture the centre first, then act, then burst.
   */
  static burstAt({ x, y }: Point, hue: string): void {
    if (Visibility.prefersReducedMotion()) return;
    for (let i = 0; i < Handoff.SPARKS; i++) {
      const spark = document.createElement("div");
      spark.className = "rad-flyer rad-flyer--spark";
      spark.style.background = hue;
      spark.style.left = `${x - 4}px`;
      spark.style.top = `${y - 4}px`;
      document.body.appendChild(spark);

      const angle = (i / Handoff.SPARKS) * Math.PI * 2;
      const distance = 34 + (i % 3) * 12;
      requestAnimationFrame(() =>
        requestAnimationFrame(() => {
          spark.style.transform = `translate(${Math.cos(angle) * distance}px,${Math.sin(angle) * distance}px) scale(.3)`;
          spark.style.opacity = "0";
        }),
      );
      setTimeout(() => spark.remove(), Handoff.BURST_MS);
    }
  }
}
