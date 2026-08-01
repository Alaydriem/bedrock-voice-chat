export interface ChatMessage {
  /** Absent for a system line. */
  author?: string;
  text: string;
  /** Rendered from the server, not typed by a player. */
  system?: boolean;
  /** Sent from the BVC app rather than typed in game. */
  fromApp?: boolean;
  /** Addressed to the local player. */
  mention?: boolean;
  /** HH:MM, stamped on arrival. */
  timestamp?: string;
}

export interface ChatOptions {
  body: HTMLElement;
  /** Player name to hue. Falls back to a neutral for anyone not in the roster. */
  hueOf: (author: string) => string;
  /** Called whenever the unread count changes while the log is closed. */
  onUnread?: (count: number) => void;
  limit?: number;
}

/**
 * Server chat, relayed live.
 *
 * History begins when you connect and ends when you leave. Nothing is stored
 * client-side, which is said in the scrollback rather than in a privacy notice,
 * because the person deciding what to type needs to know it then.
 *
 * Every author and every message body is escaped. Chat text arrives from other players
 * over the network and is the most obviously attacker-controlled string in the product;
 * the prototype interpolated it straight into `innerHTML`.
 *
 * Autoscroll only happens when the reader was already at the bottom. Yanking someone
 * away from a line they were reading because a stranger said "lol" is worse than making
 * them scroll.
 */
export class ChatLog {
  static readonly LIMIT = 200;
  static readonly STICK_PX = 40;

  readonly messages: ChatMessage[] = [];

  #options: ChatOptions;
  #open = false;
  #unread = 0;

  constructor(options: ChatOptions) {
    this.#options = options;
  }

  get unread(): number {
    return this.#unread;
  }

  get isOpen(): boolean {
    return this.#open;
  }

  setOpen(open: boolean): void {
    this.#open = open;
    if (!open) return;
    this.#unread = 0;
    this.#options.onUnread?.(0);
    this.render();
    // After the dock's height transition, not during it.
    setTimeout(() => this.scrollToEnd(), 320);
  }

  add(message: ChatMessage): void {
    this.messages.push({ ...message, timestamp: message.timestamp ?? ChatLog.stamp() });
    const limit = this.#options.limit ?? ChatLog.LIMIT;
    if (this.messages.length > limit) this.messages.shift();
    this.render();
    if (!this.#open) {
      this.#unread++;
      this.#options.onUnread?.(this.#unread);
    }
  }

  clear(): void {
    this.messages.length = 0;
    this.render();
  }

  render(): void {
    const body = this.#options.body;
    const atEnd = body.scrollTop + body.clientHeight >= body.scrollHeight - ChatLog.STICK_PX;

    const note =
      '<div class="rad-chat__note">History starts when you connect — nothing is stored</div>';
    body.innerHTML = note + this.messages.map((m) => this.#render(m)).join("");

    if (atEnd) this.scrollToEnd();
  }

  scrollToEnd(): void {
    const body = this.#options.body;
    body.scrollTop = body.scrollHeight;
  }

  static stamp(now = new Date()): string {
    return `${String(now.getHours()).padStart(2, "0")}:${String(now.getMinutes()).padStart(2, "0")}`;
  }

  static escape(text: string): string {
    const div = document.createElement("div");
    div.textContent = text;
    return div.innerHTML;
  }

  #render(m: ChatMessage): string {
    const ts = ChatLog.escape(m.timestamp ?? "");
    if (m.system) {
      return (
        '<div class="rad-msg rad-msg--system"><span class="rad-msg__avatar">·</span>' +
        `<span class="rad-msg__text">${ChatLog.escape(m.text)}</span>` +
        `<span class="rad-msg__ts">${ts}</span></div>`
      );
    }
    const author = m.author ?? "";
    const hue = this.#options.hueOf(author);
    const initial = ChatLog.escape(author.slice(0, 1).toUpperCase());
    const app = m.fromApp
      ? '<span class="rad-msg__app" title="sent from the app, not in game"></span>'
      : "";
    return (
      `<div class="rad-msg${m.mention ? " rad-msg--mention" : ""}">` +
      `<span class="rad-msg__avatar" style="background:${ChatLog.escape(hue)}">${initial}</span>` +
      '<span class="rad-msg__text">' +
      `<span class="rad-msg__author" style="color:${ChatLog.escape(hue)}">${ChatLog.escape(author)}</span>` +
      `${ChatLog.escape(m.text)}${app}</span>` +
      `<span class="rad-msg__ts">${ts}</span></div>`
    );
  }
}
