import type { KvGroup } from "./Diagnostics";

/**
 * Renders diagnostics groups and then writes values in place.
 *
 * Rebuilding `innerHTML` every second reflows the panel and reads as flicker, which
 * makes a healthy connection look unstable. The structure is built once and only text
 * nodes change afterwards; the structure is rebuilt only when the shape of the data
 * changes, which it does not during a session.
 */
export class KvGridView {
  readonly host: HTMLElement;

  #shape = "";
  #keys: HTMLElement[] = [];
  #values: HTMLElement[] = [];

  constructor(host: HTMLElement) {
    this.host = host;
  }

  update(groups: readonly KvGroup[]): void {
    const shape = groups.map((g) => `${g.title}:${g.rows.length}`).join("|");
    if (shape !== this.#shape) this.#build(groups, shape);

    let i = 0;
    for (const group of groups) {
      for (const [key, value] of group.rows) {
        if (this.#keys[i].textContent !== key) this.#keys[i].textContent = key;
        if (this.#values[i].textContent !== value) this.#values[i].textContent = value;
        i++;
      }
    }
  }

  #build(groups: readonly KvGroup[], shape: string): void {
    this.host.innerHTML = groups
      .map(
        (group) =>
          '<div class="rad-kv-group">' +
          `<div class="rad-kv-group__head">${KvGridView.#escape(group.title)}</div>` +
          group.rows
            .map(
              () =>
                '<div class="rad-kv"><span class="rad-kv__key"></span><span class="rad-kv__value"></span></div>',
            )
            .join("") +
          "</div>",
      )
      .join("");
    this.#keys = [...this.host.querySelectorAll<HTMLElement>(".rad-kv__key")];
    this.#values = [...this.host.querySelectorAll<HTMLElement>(".rad-kv__value")];
    this.#shape = shape;
  }

  static #escape(text: string): string {
    const div = document.createElement("div");
    div.textContent = text;
    return div.innerHTML;
  }
}
