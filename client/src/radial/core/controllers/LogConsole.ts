export type LogLevel = "info" | "warn" | "err";

export interface LogLine {
  timestamp: string;
  level: LogLevel;
  message: string;
}

/**
 * A streaming log, in a box.
 *
 * Capped, filtered, and it only autoscrolls when the reader is already at the bottom —
 * the same rule as chat, for the same reason: someone reading the line where it went
 * wrong should not be dragged away from it by the next heartbeat.
 *
 * Every line is escaped. Log text carries server hostnames, error strings and remote
 * responses, none of which this process wrote.
 */
export class LogConsole {
  static readonly LIMIT = 120;

  readonly host: HTMLElement;
  readonly lines: LogLine[] = [];

  #filter: LogLevel | "all" = "all";
  #limit: number;

  constructor(host: HTMLElement, limit = LogConsole.LIMIT) {
    this.host = host;
    this.#limit = limit;
  }

  get filter(): LogLevel | "all" {
    return this.#filter;
  }

  setFilter(filter: LogLevel | "all"): void {
    this.#filter = filter;
    this.render();
  }

  push(level: LogLevel, message: string, timestamp = LogConsole.stamp()): void {
    this.lines.push({ level, message, timestamp });
    if (this.lines.length > this.#limit) this.lines.shift();
    this.render();
  }

  clear(): void {
    this.lines.length = 0;
    this.render();
  }

  /** The visible lines as plain text, for a copy button. */
  toText(): string {
    return this.#visible()
      .map((l) => `${l.timestamp} ${l.level.toUpperCase().padEnd(5)} ${l.message}`)
      .join("\n");
  }

  render(): void {
    const atEnd = this.host.scrollTop + this.host.clientHeight >= this.host.scrollHeight - 24;
    this.host.innerHTML = this.#visible()
      .map(
        (l) =>
          '<div class="rad-log__line">' +
          `<span class="rad-log__ts">${LogConsole.escape(l.timestamp)}</span>` +
          `<span class="rad-log__level rad-log__level--${l.level}">${l.level}</span>` +
          `<span class="rad-log__msg">${LogConsole.escape(l.message)}</span></div>`,
      )
      .join("");
    if (atEnd) this.host.scrollTop = this.host.scrollHeight;
  }

  static stamp(now = new Date()): string {
    const p = (n: number) => String(n).padStart(2, "0");
    return `${p(now.getHours())}:${p(now.getMinutes())}:${p(now.getSeconds())}`;
  }

  static escape(text: string): string {
    const div = document.createElement("div");
    div.textContent = text;
    return div.innerHTML;
  }

  #visible(): LogLine[] {
    if (this.#filter === "all") return this.lines;
    return this.lines.filter((l) => l.level === this.#filter);
  }
}
