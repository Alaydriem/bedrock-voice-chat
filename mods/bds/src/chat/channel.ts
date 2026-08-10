import { system } from '@minecraft/server';

// `@minecraft/server-net` is never statically imported: `full` and `no-net` ship the same
// bundled main.js, and a static import breaks the no-net pack at load. bundle.js enforces it.
// Both aliases below are type-position only, so they erase at compile.
type ServerNetModule = typeof import('@minecraft/server-net');
type WebSocketClient = Awaited<
  ReturnType<ServerNetModule['websocket']['connect']>
>;

const BACKOFF_MIN_TICKS = 40;
const BACKOFF_MAX_TICKS = 1200;
const LIVENESS_TICKS = 100;

/**
 * The mod's chat channel to the BVC server — one socket, both directions.
 *
 * Text frames of JSON tagged on `t`, matching `common::structs::chat::ChatFrame`. `hello` is
 * always first and names the world; nothing after it carries one, because the world is a
 * property of the connection.
 */
export class ChatChannel {
  private client: WebSocketClient | null = null;
  private connecting = false;
  private stopped = false;
  private backoff = BACKOFF_MIN_TICKS;
  private liveness: number | null = null;
  private retryPending = false;

  constructor(
    private readonly serverUrl: string,
    private readonly accessToken: string,
    private readonly worldUuid: string,
    private readonly worldName: string,
    private readonly onSay: (author: string, text: string) => void,
  ) {}

  get isOpen(): boolean {
    return this.client !== null && this.client.isOpen;
  }

  start(): void {
    this.stopped = false;
    void this.connect();

    // This client exposes no ping API, so liveness is `isOpen` plus the close event.
    //
    // It is a backstop for a socket that died without a close event, never a second dialler:
    // a retry already in flight owns the reconnect, and dialling alongside it is what turns
    // one dropped connection into several live ones.
    this.liveness = system.runInterval(() => {
      if (this.stopped || this.connecting || this.retryPending || this.isOpen) {
        return;
      }
      void this.connect();
    }, LIVENESS_TICKS);
  }

  stop(): void {
    this.stopped = true;
    if (this.liveness !== null) {
      system.clearRun(this.liveness);
      this.liveness = null;
    }
    const current = this.client;
    this.client = null;
    this.discard(current);
  }

  /**
   * Reports a line a player typed in game.
   *
   * Nothing is queued while the socket is down. A backlog delivered on reconnect drops stale
   * lines into a conversation that has moved on, and no message is ever persisted.
   */
  report(author: string, text: string): void {
    if (!this.isOpen) {
      return;
    }
    this.send({ t: 'chat', author, text });
  }

  /**
   * Reports something the server said: a death, a join, a leave.
   *
   * Carries no author, which is what makes it render as a system line rather than as a player
   * speaking. Dropped while the socket is down for the same reason a chat line is.
   */
  event(text: string): void {
    if (!this.isOpen) {
      return;
    }
    this.send({ t: 'event', text });
  }

  private async connect(): Promise<void> {
    if (this.connecting || this.stopped) {
      return;
    }
    this.connecting = true;

    try {
      const net = await this.loadNet();
      if (!net) {
        // No network module at all: this is a no-net world, where the client-local path owns
        // chat and this channel has no part to play.
        this.stopped = true;
        return;
      }

      const url = this.serverUrl.replace(/^http/, 'ws') + '/api/websocket/chat';
      const client = await net.websocket.connect(url, [
        new net.HttpHeader('X-MC-Access-Token', this.accessToken),
      ]);

      if (this.stopped) {
        this.discard(client);
        return;
      }

      // Whatever was here is no longer the channel. Left open it would stay registered on the
      // server until the server displaced it, and its eventual close would be read as this
      // one's. Installed before the old one is closed, so that close is recognised as stale.
      const previous = this.client;
      this.client = client;
      this.discard(previous);
      this.backoff = BACKOFF_MIN_TICKS;

      client.afterEvents.message.subscribe((e) => this.receive(e.message));
      client.afterEvents.close.subscribe((e) => {
        // A socket already replaced says nothing about the current one. Acting on its close
        // both discards a live client and dials again, which is how one drop becomes many
        // connections.
        if (this.client !== client) {
          return;
        }
        console.warn('[BVC] chat channel closed: ' + e.reason);
        this.client = null;
        this.scheduleRetry();
      });

      this.send({
        t: 'hello',
        world: this.worldUuid,
        world_name: this.worldName,
        game: 'minecraft',
      });
      console.info('[BVC] chat channel connected');
    } catch (e) {
      console.warn('[BVC] chat channel connect failed: ' + e);
      this.scheduleRetry();
    } finally {
      this.connecting = false;
    }
  }

  private receive(body: string): void {
    try {
      const frame = JSON.parse(body) as {
        t?: string;
        author?: string;
        text?: string;
      };
      if (frame.t !== 'say' || !frame.author || !frame.text) {
        return;
      }
      this.onSay(frame.author, frame.text);
    } catch {
      console.warn('[BVC] chat channel received an undecodable frame');
    }
  }

  private send(frame: Record<string, string>): void {
    try {
      this.client?.send(JSON.stringify(frame));
    } catch (e) {
      console.warn('[BVC] chat channel send failed: ' + e);
    }
  }

  /**
   * Arranges one redial.
   *
   * At most one is ever outstanding. Every socket that dies asks for a retry, and a world
   * holding several sockets therefore asks several times for the one connection it wants;
   * honouring each request builds a socket per request and the count only climbs.
   */
  private scheduleRetry(): void {
    if (this.stopped || this.retryPending) {
      return;
    }
    this.retryPending = true;

    // Jitter is load-bearing: without it a BVC server restart has every world's mod redial in
    // lockstep, and they all fail together again.
    const jitter = Math.floor(Math.random() * BACKOFF_MIN_TICKS);
    const delay = Math.min(this.backoff, BACKOFF_MAX_TICKS) + jitter;
    this.backoff = Math.min(this.backoff * 2, BACKOFF_MAX_TICKS);
    system.runTimeout(() => {
      this.retryPending = false;
      void this.connect();
    }, delay);
  }

  /** Closes a socket this channel is done with. A closed one throws rather than reporting. */
  private discard(client: WebSocketClient | null): void {
    if (!client || !client.isOpen) {
      return;
    }
    try {
      client.close();
    } catch (e) {
      console.warn('[BVC] chat channel close failed: ' + e);
    }
  }

  private async loadNet(): Promise<ServerNetModule | null> {
    try {
      return (await import('@minecraft/server-net')) as unknown as ServerNetModule;
    } catch {
      return null;
    }
  }
}
