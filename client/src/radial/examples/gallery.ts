import { Mount } from "$radial/bindings/Mount";
import { Color } from "$radial/core/color/Color";

interface Page {
  href: string;
  label: string;
}

/**
 * Shared chrome for the reference pages.
 *
 * The navigation and the device toggle are defined once here rather than copied into
 * eleven HTML files, so adding a page is one entry in PAGES.
 */
export class Gallery {
  static readonly PAGES: readonly Page[] = [
    { href: "index.html", label: "Overview" },
    { href: "controls.html", label: "Controls" },
    { href: "containers.html", label: "Containers" },
    { href: "overlays.html", label: "Overlays" },
    { href: "loader.html", label: "Loader" },
    // Introduction, the gate, sign-in and the dead end are one flow in one frame, so
    // they are one page — splitting them would hide the routing between them, which is
    // the part worth reviewing.
    { href: "onboarding.html", label: "Onboarding" },
    { href: "servers.html", label: "Servers" },
    { href: "dashboard.html", label: "Dashboard" },
    { href: "settings.html", label: "Settings" },
  ];

  /**
   * An element that has to exist, typed as non-null.
   *
   * `getElementById` plus a throw narrows at the top level but not inside a hoisted
   * function declaration, because such a function could in principle run before the
   * throw. Returning the narrowed type from a call sidesteps that entirely.
   */
  static requireElement(id: string): HTMLElement {
    const el = document.getElementById(id);
    if (!el) throw new Error(`radial examples: #${id} is missing from the page`);
    return el;
  }

  /** Same, for a selector inside a known root. */
  static require<T extends HTMLElement>(root: ParentNode, selector: string): T {
    const el = root.querySelector<T>(selector);
    if (!el) throw new Error(`radial examples: ${selector} is missing from the page`);
    return el;
  }

  /** Render the page nav into every `[data-g-nav]`, marking the current page. */
  static nav(): void {
    const here = (location.pathname.split("/").pop() || "index.html").toLowerCase();
    const html = Gallery.PAGES.map((p) => {
      const current = p.href.toLowerCase() === here ? ' aria-current="page"' : "";
      return `<a href="${p.href}"${current}>${p.label}</a>`;
    }).join("");
    for (const el of document.querySelectorAll<HTMLElement>("[data-g-nav]")) {
      el.innerHTML = html;
    }
  }

  /**
   * Wire a desktop/phone toggle to a frame.
   *
   * The frame is a container query root, so switching its width is genuinely all that
   * happens — every component inside re-lays itself out because its container changed,
   * not because a media query fired.
   */
  static deviceToggle(group: HTMLElement, frame: HTMLElement): void {
    const buttons = [...group.querySelectorAll<HTMLButtonElement>("button[data-g-size]")];
    const apply = (size: string) => {
      frame.classList.toggle("rad-frame--phone", size === "phone");
      frame.classList.toggle("rad-frame--desktop", size !== "phone");
      for (const b of buttons) b.setAttribute("aria-pressed", String(b.dataset.gSize === size));
    };
    for (const b of buttons) {
      b.addEventListener("click", () => apply(b.dataset.gSize ?? "desktop"));
    }
    apply(buttons.find((b) => b.getAttribute("aria-pressed") === "true")?.dataset.gSize ?? "desktop");
  }

  /**
   * Fill `[data-g-contrast]` with the measured ratio against the frame ground.
   *
   * Measured, not asserted. The design notes claim 9.9:1 for body text and 6.6:1 for
   * the mono labels; a table that computes them is the only version of that claim that
   * cannot quietly go stale.
   */
  static contrast(): void {
    for (const el of document.querySelectorAll<HTMLElement>("[data-g-contrast]")) {
      const [fg, bg] = (el.dataset.gContrast ?? "").split("|");
      if (!fg || !bg) continue;
      const ratio = Color.contrast(fg, bg);
      const min = Number.parseFloat(el.dataset.gContrastMin ?? "4.5");
      el.textContent = `${ratio.toFixed(2)}:1`;
      el.classList.add(ratio >= min ? "g-pass" : "g-fail");
    }
  }

  /** Nav, contrast readouts, and every `data-rad-*` binding on the page. */
  static boot(): Mount {
    Gallery.nav();
    Gallery.contrast();
    for (const group of document.querySelectorAll<HTMLElement>("[data-g-device]")) {
      const target = document.querySelector<HTMLElement>(`#${group.dataset.gDevice}`);
      if (target) Gallery.deviceToggle(group, target);
    }
    return Mount.scan(document);
  }
}
