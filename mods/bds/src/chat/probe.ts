import { system } from '@minecraft/server';
import { serverAdminConfig } from '../config';
import { httpClient } from '../net';
import { ProbeTiming } from './probe_timing';

// `@minecraft/server-net` is never statically imported: `full` and `no-net` ship the same
// bundled main.js, and a static import breaks the no-net pack at load. bundle.js enforces
// this. Both aliases below are type-position only, so they erase at compile.
type ServerNetModule = typeof import('@minecraft/server-net');
type WebSocketClient = Awaited<
  ReturnType<ServerNetModule['websocket']['connect']>
>;

// Temporary. Deleted in Task 13, when the real ChatChannel replaces it.
//
// Answers the question that gates the whole net-mode design: does holding a websocket open
// starve the HTTP requests that share @minecraft/server-net? Positions ride that module and
// positions are the audio hot path, so a yes means the transport gets redesigned.
//
// Runs the entire experiment from one console command and needs no Minecraft client — the
// position loop only fires with a player connected, and `bvc_minimum_players` cannot be set
// below 1. So this drives its own HTTP load against the unauthenticated /health/liveness
// endpoint at the same 5-tick cadence the position loop uses.
//
//   scriptevent bvc:probe                          # BDS console, no leading slash
//   scriptevent bvc:probe wss://echo.websocket.org # override the socket endpoint
//   scriptevent bvc:probeoff                       # abandon a run in progress
//
// `wss://` is mandatory: a plain `ws://` URI raises TLSOnlyError, and a host outside the
// server's allow list raises UriNotAllowedError. Both are caught and named rather than swallowed.
export class ChatProbe {
  private static readonly DEFAULT_WS = 'wss://echo.websocket.org';
  // Matches POLL_INTERVAL in main.ts, so the load looks like the real position loop.
  private static readonly SAMPLE_TICKS = 5;
  private static readonly BASELINE_TICKS = 200;
  private static readonly HOLD_TICKS = 600;
  private static readonly REQUEST_TIMEOUT_SEC = 1;

  private static client: WebSocketClient | null = null;
  private static sampler: number | null = null;
  private static running = false;
  private static abort = false;

  static register(): void {
    system.afterEvents.scriptEventReceive.subscribe(
      (event) => {
        if (event.id === 'bvc:probe') {
          void ChatProbe.run(event.message.trim());
        } else if (event.id === 'bvc:probeoff') {
          ChatProbe.abort = true;
          ChatProbe.closeSocket('operator');
        }
      },
      { namespaces: ['bvc'] },
    );
    console.info('[BVC probe] ready — run: scriptevent bvc:probe');
  }

  static async run(url: string): Promise<void> {
    if (ChatProbe.running) {
      console.warn('[BVC probe] already running');
      return;
    }

    const target = url.length > 0 ? url : ChatProbe.DEFAULT_WS;
    if (!target.startsWith('wss://')) {
      console.error(`[BVC probe] refusing "${target}" — wss:// is required (TLSOnlyError)`);
      return;
    }
    if (!serverAdminConfig.bvcServer) {
      console.error('[BVC probe] bvc_server is not configured; nothing to time against');
      return;
    }

    ChatProbe.running = true;
    ChatProbe.abort = false;

    try {
      console.info('[BVC probe] phase 1/3 — baseline, socket shut (10s)');
      const before = await ChatProbe.sample('before', ChatProbe.BASELINE_TICKS);
      if (ChatProbe.abort) {
        return;
      }

      console.info(`[BVC probe] phase 2/3 — opening ${target}`);
      const opened = await ChatProbe.openSocket(target);
      if (!opened) {
        console.error('[BVC probe] aborted: socket did not open');
        console.info(`[BVC probe] ${before.summary()}`);
        return;
      }

      console.info('[BVC probe] phase 2/3 — socket OPEN, sampling (30s)');
      const during = await ChatProbe.sample('during (ws open)', ChatProbe.HOLD_TICKS);

      ChatProbe.closeSocket('hold elapsed');
      console.info('[BVC probe] phase 3/3 — socket shut, sampling (10s)');
      const after = await ChatProbe.sample('after', ChatProbe.BASELINE_TICKS);

      console.info('[BVC probe] ================ RESULT ================');
      console.info(`[BVC probe] ${before.summary()}`);
      console.info(`[BVC probe] ${during.summary()}`);
      console.info(`[BVC probe] ${after.summary()}`);
      console.info(
        '[BVC probe] PASS if "during" mean/p95 is comparable to before and after, and ' +
          'fail=0 throughout. A large p95 or any failures means the socket starves HTTP.',
      );
      console.info('[BVC probe] ========================================');
    } finally {
      ChatProbe.closeSocket('run ended');
      ChatProbe.stopSampler();
      ChatProbe.running = false;
    }
  }

  // Issues a GET every SAMPLE_TICKS and resolves once `ticks` have elapsed. Requests are not
  // awaited in series: the position loop does not serialise either, and serialising would hide
  // exactly the contention this is looking for.
  private static sample(label: string, ticks: number): Promise<ProbeTiming> {
    const timing = new ProbeTiming(label);
    const url = `${serverAdminConfig.bvcServer}/health/liveness`;

    return new Promise((resolve) => {
      let elapsed = 0;

      ChatProbe.sampler = system.runInterval(() => {
        elapsed += ChatProbe.SAMPLE_TICKS;

        const startedAt = Date.now();
        void httpClient
          .request(url, 'Get', undefined, [['Accept', 'application/json']], ChatProbe.REQUEST_TIMEOUT_SEC)
          .then((response) => {
            const ok = response !== null && response.status >= 200 && response.status < 400;
            timing.record(Date.now() - startedAt, ok);
          });

        if (elapsed >= ticks || ChatProbe.abort) {
          ChatProbe.stopSampler();
          // One tick of slack so the last in-flight request can land before summarising.
          system.runTimeout(() => resolve(timing), 20);
        }
      }, ChatProbe.SAMPLE_TICKS);
    });
  }

  private static async openSocket(target: string): Promise<boolean> {
    const net = await ChatProbe.loadNet();
    if (!net) {
      console.error('[BVC probe] @minecraft/server-net unavailable; this is a no-net server');
      return false;
    }

    try {
      const client = await net.websocket.connect(target, [
        new net.HttpHeader('X-MC-Access-Token', serverAdminConfig.accessToken),
      ]);
      ChatProbe.client = client;
      console.info(`[BVC probe] connected, isOpen=${client.isOpen}`);

      client.afterEvents.close.subscribe((e) => {
        console.warn(`[BVC probe] socket closed by peer, reason=${e.reason}`);
        ChatProbe.client = null;
      });

      // Kept deliberately quiet. Echoing traffic would measure the echo server rather than
      // the module, and a chat socket is near-idle in normal use anyway.
      return true;
    } catch (e) {
      // The error class is the finding: TLSOnlyError means the scheme, UriNotAllowedError
      // means the allow list, WebSocketConnectionFailedError means the endpoint refused.
      const name = e instanceof Error ? e.constructor.name : typeof e;
      console.error(`[BVC probe] connect FAILED (${name}): ${e}`);
      return false;
    }
  }

  private static closeSocket(why: string): void {
    if (ChatProbe.client && ChatProbe.client.isOpen) {
      ChatProbe.client.close();
      console.info(`[BVC probe] socket closed (${why})`);
    }
    ChatProbe.client = null;
  }

  private static stopSampler(): void {
    if (ChatProbe.sampler !== null) {
      system.clearRun(ChatProbe.sampler);
      ChatProbe.sampler = null;
    }
  }

  private static async loadNet(): Promise<ServerNetModule | null> {
    try {
      return (await import('@minecraft/server-net')) as unknown as ServerNetModule;
    } catch {
      return null;
    }
  }
}
