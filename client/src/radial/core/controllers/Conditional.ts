/**
 * Panes that appear in response to a choice.
 *
 * The prototype had three bespoke rules — `ws-only`, `proxy-only`, `realm-only` —
 * each needing its own CSS. One attribute replaces them:
 *
 *   <div class="rad-frame" data-rad-state="realm">
 *     <div data-rad-when="proxy realm">Shown for either.</div>
 *     <div data-rad-when="direct">Shown only when direct.</div>
 *
 * Realms is a routing question, not a settings page: what the operator is choosing is
 * where position data comes from, so the page leads with the choice and everything
 * else appears in response to it.
 */
export class Conditional {
  readonly frame: HTMLElement;

  constructor(frame: HTMLElement) {
    this.frame = frame;
    this.apply();
  }

  get state(): string {
    return this.frame.dataset.radState ?? "";
  }

  set state(next: string) {
    this.frame.dataset.radState = next;
    this.apply();
  }

  /**
   * Hide the panes that do not match, and mark the frame resolved. Until this has
   * run, CSS hides every conditional pane — otherwise the proxy and realm panes both
   * flash on load.
   */
  apply(): void {
    const active = new Set(this.state.split(/\s+/).filter(Boolean));
    for (const el of this.frame.querySelectorAll<HTMLElement>("[data-rad-when]")) {
      const wanted = (el.dataset.radWhen ?? "").split(/\s+/).filter(Boolean);
      el.hidden = !wanted.some((w) => active.has(w));
    }
    this.frame.dataset.radResolved = "true";
  }
}
