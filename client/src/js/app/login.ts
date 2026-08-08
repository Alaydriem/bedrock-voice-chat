import { I18n } from "$lib/i18n";
import { fetch } from '@tauri-apps/plugin-http';
import { info, error, warn } from '@tauri-apps/plugin-log';
import { Store } from '@tauri-apps/plugin-store';
import { openUrl } from '@tauri-apps/plugin-opener';
import { getVersion } from '@tauri-apps/api/app';
import { invoke } from '@tauri-apps/api/core';
import { writable, derived, get, type Writable, type Readable } from 'svelte/store';
import { stopForegroundService, isServiceRunning } from 'tauri-plugin-audio-permissions';
import { ServerListStore } from './services/ServerListStore';
import MinecraftAuthUrl from './auth/MinecraftAuthUrl';
import FinishWatchdog from './login/FinishWatchdog';
import BVCApp from './BVCApp.ts';
import HelpLinks from './HelpLinks';
import Analytics from './analytics';
import PlatformDetector from './utils/PlatformDetector.ts';
import { AddServerRoute } from './server/AddServerRoute';
import type { HytaleDeviceFlowStartResponse, HytaleDeviceFlowStatusResponse, LoginResponse, ServerListEntry } from '../bindings/index.ts';
import type { LoginPageState } from './login/LoginPageState';
import { AppStore } from "./services/AppStore";

declare global {
  interface Window {
    App: any;
  }
}

/**
 * Outcome of a login attempt, used by the login page to drive its
 * idle -> connecting -> error/redirect view state.
 *
 * - `invalid`     pre-flight validation failed; an inline error has been set
 *                 via the formError store and the page should stay on idle.
 * - `navigating`  the page is performing a same-window navigation (code login).
 * - `redirecting` the attempt succeeded and an external auth URL was opened;
 *                 the page should remain in its "connecting" handoff state.
 * - `error`       the server could not be reached or denied access; the page
 *                 should present the connection-error view for `sanitized`.
 */
export type LoginAttemptResult =
  | { status: 'invalid'; sanitized?: string }
  | { status: 'navigating'; sanitized: string }
  // `authUrl` is the address handed to the system browser. Carried back so the
  // handoff screen can offer it for copying when no browser window appeared.
  | { status: 'redirecting'; sanitized: string; authUrl?: string }
  | { status: 'error'; sanitized: string };

/**
 * Status messages cycled beneath the spinner while a connection attempt is in
 * flight. Edit this list freely - it is the single source of truth for the
 * "connecting" fidget copy.
 */
export const CONNECTING_STATUS_PHRASES: readonly string[] = [
  'Reaching your server…',
  'Verifying server configuration…',
  'Checking your permissions…',
  'Negotiating a secure channel…',
  'Almost there…',
];

/**
 * Connection-flow view state the login page renders: the editable form
 * (`idle`), the spinner/handoff view (`connecting`), or the connection-error
 * card (`error`).
 */
export type LoginState = 'idle' | 'connecting' | 'error';

/**
 * Which sign-in path an attempt uses. `ms` opens the Microsoft OAuth flow in
 * the external browser; `hytale` starts the device-code flow.
 */
export type LoginKind = 'ms' | 'hytale';

export default class Login extends BVCApp {
  private static readonly CODE_LOGIN_PATTERN = /^(https?:\/\/)?code@/;

  // Key under which the deep-link auth callback persists a user-facing error
  // when login fails after returning from the external browser. The login page
  // observes this key so failures surface as a warning instead of leaving the
  // user on a frozen "connecting" view.
  static readonly LOGIN_ERROR_KEY = "login_error";


  readonly CONFIG_ENDPOINT = "/api/config";
  readonly AUTH_ENDPOINT = "/api/auth";
  readonly NCRYPTF_EK_ENDPOINT = "/ncryptf/ek";

  private hytalePollingInterval: number | null = null;

  private isMobileStore: Writable<boolean>;
  private appVersionStore: Writable<string>;
  private formErrorStore: Writable<string>;
  private serverInputInvalidStore: Writable<boolean>;
  private serverInputStore: Writable<string>;
  private loginStateStore: Writable<LoginState>;
  private attemptServerStore: Writable<string>;
  private isHandoffStore: Writable<boolean>;
  private isFinishingStore: Writable<boolean>;
  private authUrlStore: Writable<string>;

  public readonly isMobileReadable: Readable<boolean>;
  public readonly appVersionReadable: Readable<string>;
  public readonly formError: Readable<string>;
  public readonly serverInputInvalid: Readable<boolean>;
  public readonly serverInput: Readable<string>;
  public readonly isCodeLogin: Readable<boolean>;
  public readonly loginState: Readable<LoginState>;
  public readonly attemptServer: Readable<string>;
  public readonly isHandoff: Readable<boolean>;
  public readonly isFinishing: Readable<boolean>;
  /** The address handed to the browser, so the handoff screen can offer it. */
  public readonly authUrl: Readable<string>;

  // Monotonic token identifying the current attempt. Bumped on each new attempt
  // and on cancel so a late-resolving request can't clobber the UI.
  private attemptId = 0;
  private lastLoginKind: LoginKind = 'ms';
  // Tracks a genuine background->foreground transition during handoff. Only a
  // real app resume (mobile) should start the finishing flow; a desktop window
  // that merely keeps focus while the browser is open must not.
  private wasHiddenDuringHandoff = false;
  private readonly finishWatchdog: FinishWatchdog;

  /**
   * Polls for a callback that landed without announcing itself.
   *
   * The handoff view has exactly one way out on its own: `deep-link-received` fires, the router
   * routes it, and the handler navigates. Nothing guarantees that event is heard — on Android the
   * intent arrives while the activity is being recreated, so it can be emitted before this page's
   * listener exists or after a teardown has released it. The redemption then never starts, and the
   * screen sits on "continue in the window that just opened" having already been signed in.
   *
   * `pending_deep_link` is the durable half of the same delivery and is written for every intent,
   * so asking the store on a timer closes that hole without depending on any event at all. It
   * runs only while the handoff is showing, which is the only time an unheard callback is what is
   * being waited for.
   */
  private static readonly PENDING_POLL_MS = 1_500;
  private pendingPoll: ReturnType<typeof setInterval> | null = null;

  private platformDetector: PlatformDetector;

  constructor() {
    super();
    this.isMobileStore = writable(false);
    this.appVersionStore = writable('');
    this.formErrorStore = writable('');
    this.serverInputInvalidStore = writable(false);
    this.serverInputStore = writable('');
    this.loginStateStore = writable<LoginState>('idle');
    this.attemptServerStore = writable('');
    this.isHandoffStore = writable(false);
    this.isFinishingStore = writable(false);
    this.authUrlStore = writable('');
    this.isMobileReadable = { subscribe: this.isMobileStore.subscribe };
    this.appVersionReadable = { subscribe: this.appVersionStore.subscribe };
    this.formError = { subscribe: this.formErrorStore.subscribe };
    this.serverInputInvalid = { subscribe: this.serverInputInvalidStore.subscribe };
    this.serverInput = { subscribe: this.serverInputStore.subscribe };
    this.isCodeLogin = derived(this.serverInputStore, (value) => Login.isCodeLoginInput(value));
    this.loginState = { subscribe: this.loginStateStore.subscribe };
    this.attemptServer = { subscribe: this.attemptServerStore.subscribe };
    this.isHandoff = { subscribe: this.isHandoffStore.subscribe };
    this.isFinishing = { subscribe: this.isFinishingStore.subscribe };
    this.authUrl = { subscribe: this.authUrlStore.subscribe };
    this.platformDetector = new PlatformDetector();
    this.finishWatchdog = new FinishWatchdog(
      () => get(this.loginStateStore) === 'connecting',
      () => this.callbackInFlight(),
      () => {
        this.stopPendingPoll();
        this.isHandoffStore.set(false);
        this.isFinishingStore.set(false);
        this.loginStateStore.set('error');
      },
      info,
    );
  }

  async initialize() {
    await this.initializeDeepLinks();
  }

  static isCodeLoginInput(value: string): boolean {
    return Login.CODE_LOGIN_PATTERN.test(value);
  }

  async initializePage(): Promise<LoginPageState> {
    this.isMobileStore.set(await this.platformDetector.checkMobile());
    this.appVersionStore.set(await getVersion());

    const urlParams = new URLSearchParams(window.location.search);
    if (urlParams.get('logout') === 'true') {
      await this.escapeHatchLogout();
    }

    await invoke('reset_asm').catch(() => {});

    await this.initialize();
    this.preloader();

    const isAddServer = urlParams.has('addserver');
    const { href: backHref, label: backLabel } = AddServerRoute.backFrom(urlParams);

    const prefilledServer = urlParams.get('server') ?? '';
    const autoReauth = urlParams.has('server') && urlParams.get('reauth') === 'true';

    return { isAddServer, backHref, backLabel, prefilledServer, autoReauth };
  }

  // Bounds each escape-hatch teardown step. The user reached the hatch
  // because a subsystem hung; an unbounded teardown of that same subsystem
  // would strand them here a second time.
  private static readonly ESCAPE_LOGOUT_STEP_TIMEOUT_MS = 5000;

  private withEscapeTimeout<T>(work: Promise<T>, label: string): Promise<T> {
    return Promise.race([
      work,
      new Promise<T>((_, reject) =>
        setTimeout(
          () => reject(new Error(`${label} timed out`)),
          Login.ESCAPE_LOGOUT_STEP_TIMEOUT_MS,
        ),
      ),
    ]);
  }

  /**
   * Signs the user out after arriving from the preloader escape hatch
   * (`/login?logout=true`). The originating page hung mid-init, so audio and
   * network streams may still be running; stop them best-effort before
   * clearing credentials.
   */
  private async escapeHatchLogout(): Promise<void> {
    // Strip the parameter first so a refresh during a wedged teardown lands
    // on a clean login page instead of re-entering the same hang.
    window.history.replaceState({}, '', '/login');

    try { await this.withEscapeTimeout(invoke("stop_audio_device", { device: "InputDevice" }), "stop input"); } catch (_) {}
    try { await this.withEscapeTimeout(invoke("stop_audio_device", { device: "OutputDevice" }), "stop output"); } catch (_) {}
    try { await this.withEscapeTimeout(invoke("stop_network_stream"), "stop network"); } catch (_) {}

    if (await this.platformDetector.checkMobile()) {
      try {
        const status = await this.withEscapeTimeout(isServiceRunning(), "service status");
        if (status.running) {
          await this.withEscapeTimeout(stopForegroundService(), "stop service");
        }
      } catch (_) {}
    }

    try {
      Analytics.track("Logout");
      await this.withEscapeTimeout(invoke("logout"), "logout");
    } catch (e) {
      warn(`Escape hatch logout failed: ${e}`);
    }
  }

  /**
   * The server portion of a `code@<host>` value.
   *
   * The code flow is a screen rather than a route, so this returns the address and
   * navigates nowhere: the page reacts to `isCodeLogin`.
   */
  codeLoginServer(rawValue: string): string {
    const server = rawValue.replace(/^(https?:\/\/)?code@/, (_, proto) => proto ?? 'https://');
    return this.sanitizeServerUrl(server);
  }

  /**
   * The server a `code@<host>` entry points at, or null while the host is not yet a
   * usable address.
   *
   * `code@` is a prefix, so it matches from the moment the `@` is typed and keeps
   * matching for every keystroke of the host after it. Null is what distinguishes
   * "the user is still typing" from "this is a complete instruction", and it has to be
   * answered before anything acts on the value: a server URL built from a half-typed
   * host is a different server.
   *
   * Pure, with no error reported and no store touched, so a view can ask it on every
   * keystroke to describe the field.
   */
  codeLoginTarget(rawValue: string): string | null {
    if (!Login.isCodeLoginInput(rawValue)) return null;
    const host = rawValue.trim().replace(/^(https?:\/\/)?code@/, '');
    return this.validateServerUrl(host).valid ? this.sanitizeServerUrl(host) : null;
  }

  public sanitizeServerUrl(url: string): string {
    let sanitized = url.trim();

    if (!sanitized.startsWith("https://")) {
      if (sanitized.startsWith("http://")) {
        sanitized = sanitized.replace("http://", "https://");
      } else {
        sanitized = "https://" + sanitized;
      }
    }

    if (sanitized.endsWith("/")) {
      sanitized = sanitized.slice(0, -1);
    }

    return sanitized;
  }

  public validateServerUrl(url: string): { valid: boolean; error?: string } {
    const trimmed = url.trim();

    if (!trimmed) {
      return { valid: false, error: I18n.t("Please enter a server URL") };
    }

    const sanitized = this.sanitizeServerUrl(trimmed);
    try {
      const parsed = new URL(sanitized);
      if (parsed.protocol !== "https:") {
        return { valid: false, error: I18n.t("Server URL must use HTTPS") };
      }
      if (!parsed.hostname || parsed.hostname.length < 3) {
        return { valid: false, error: I18n.t("Please enter a valid server URL") };
      }
    } catch {
      return { valid: false, error: I18n.t("Please enter a valid server URL") };
    }

    return { valid: true };
  }

  public clearError(): void {
    this.formErrorStore.set('');
    this.serverInputInvalidStore.set(false);
  }

  /**
   * Read and clear any auth-callback error left by the deep-link handler. Used
   * on page mount to surface failures from a callback that completed before the
   * page (re)loaded (e.g. a cold-started OAuth return).
   */
  public async consumeAuthError(): Promise<string | null> {
    const store = await this.getStore();
    const message = await store.get<string>(Login.LOGIN_ERROR_KEY);
    if (message) {
      await store.delete(Login.LOGIN_ERROR_KEY);
      await store.save();
      return message;
    }
    return null;
  }

  /**
   * Observe auth-callback errors written by the deep-link handler while the
   * page is already mounted (the warm OAuth return path). Fires the callback
   * with the message; the caller is responsible for clearing it via
   * consumeAuthError so it is not replayed on the next mount.
   */
  public async watchAuthError(cb: (message: string) => void): Promise<() => void> {
    const store = await this.getStore();
    return await store.onKeyChange<string>(Login.LOGIN_ERROR_KEY, (value) => {
      if (value) cb(value);
    });
  }

  /**
   * GET fetch with a hard timeout.
   *
   * We use a manually-cleared AbortController instead of AbortSignal.timeout()
   * on purpose. The Tauri http plugin registers an `abort` listener on the
   * signal but never removes it after the request completes, and its abort
   * handler calls `plugin:http|fetch_cancel` against the request's resource id.
   * With AbortSignal.timeout(ms) the abort event fires unconditionally once the
   * timer elapses - even when the request already succeeded - so fetch_cancel
   * runs against a resource id that has already been consumed/dropped. That
   * surfaces as an uncaught "The resource id <n> is invalid." promise rejection
   * (the very errors flooding Sentry). Clearing the timer on completion means
   * abort() only ever fires while the request is genuinely in flight.
   */
  private async fetchWithTimeout(url: string, timeoutMs = 5000): Promise<Response> {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), timeoutMs);
    try {
      return await fetch(url, {
        signal: controller.signal,
        method: 'GET',
      });
    } finally {
      clearTimeout(timeoutId);
    }
  }

  public reportError(message: string): void {
    this.formErrorStore.set(message);
    this.serverInputInvalidStore.set(true);

    // A non-empty error arriving mid-flow (e.g. a Hytale device-flow poll
    // expiring or being denied after we've handed off to the browser) must not
    // be swallowed by the connecting view - drop back to idle so the inline
    // message is actually visible.
    if (get(this.loginStateStore) !== 'idle') {
      this.clearFinishWatchdog();
      this.isHandoffStore.set(false);
      this.isFinishingStore.set(false);
      this.loginStateStore.set('idle');
    }
  }

  /**
   * Cancel an in-progress Hytale device-flow poll. Used when the user backs
   * out of the "connecting" handoff view so we don't keep polling a flow they
   * abandoned.
   */
  public cancelHytalePolling(): void {
    this.stopHytalePolling();
  }

  public setServerInput(value: string): void {
    this.serverInputStore.set(value);
  }

  public blurServerInput(): void {
    const value = get(this.serverInputStore);
    if (value.trim()) {
      this.serverInputStore.set(this.sanitizeServerUrl(value));
    }
  }


  /**
   * Drive an attempt through the idle -> connecting -> redirect/error flow.
   * Code-login input short-circuits to a same-window navigation; everything
   * else validates, flips to the connecting view, and applies the outcome -
   * guarded by an attempt token so a stale request can't clobber a newer one.
   */
  async runLogin(kind: LoginKind): Promise<void> {
    if (get(this.loginStateStore) === 'connecting') return;

    this.lastLoginKind = kind;
    const value = get(this.loginStateStore) === 'error'
      ? get(this.attemptServerStore)
      : get(this.serverInputStore);

    // Code-login is a screen the page has already switched to, so there is no
    // attempt to make here. Returning early also keeps it out of the connecting
    // view, which it would otherwise be stranded on.
    if (Login.isCodeLoginInput(value)) {
      return;
    }

    // Validate before flipping to the connecting view so bad/empty input shows
    // the inline error without a connecting flicker.
    const validation = this.validateServerUrl(value);
    if (!validation.valid) {
      this.reportError(validation.error || I18n.t("Please enter a valid server URL"));
      this.loginStateStore.set('idle');
      return;
    }

    this.clearError();
    const sanitized = this.sanitizeServerUrl(value);
    this.attemptServerStore.set(sanitized);
    this.serverInputStore.set(sanitized);
    this.clearFinishWatchdog();
    this.isHandoffStore.set(false);
    this.isFinishingStore.set(false);
    this.wasHiddenDuringHandoff = false;
    this.loginStateStore.set('connecting');

    const myAttempt = ++this.attemptId;
    const result = kind === 'hytale'
      ? await this.loginWithHytale(value)
      : await this.login(value);

    // The user cancelled (or started a fresh attempt) while this one was in
    // flight - don't reapply its result. Undo any polling it may have kicked off.
    if (myAttempt !== this.attemptId) {
      this.cancelHytalePolling();
      return;
    }

    this.applyResult(result);
  }

  async tryAgain(): Promise<void> {
    await this.runLogin(this.lastLoginKind);
  }

  /**
   * Auto-reauthenticate via Microsoft on page load when requested, but only if
   * the page is still idle and no error is showing (a pending auth-callback
   * failure surfaced on mount must not be steamrolled by a fresh attempt).
   */
  async attemptAutoReauth(): Promise<void> {
    if (get(this.loginStateStore) === 'idle' && !get(this.formErrorStore)) {
      await this.runLogin('ms');
    }
  }

  /**
   * Manual escape from the connecting/handoff view (e.g. the user closed the
   * external sign-in window without finishing, or chose a different server).
   * Stops any Hytale polling and clears transient errors before returning to
   * the editable idle form.
   */
  public returnToIdle(): void {
    this.attemptId++;
    this.cancelHytalePolling();
    this.clearError();
    this.clearFinishWatchdog();
    this.isHandoffStore.set(false);
    this.isFinishingStore.set(false);
    this.wasHiddenDuringHandoff = false;
    this.loginStateStore.set('idle');
    this.serverInputStore.set(get(this.attemptServerStore));
  }

  /**
   * Surface an auth-callback failure persisted by the deep-link handler: stop
   * any finishing/handoff view and drop back to the editable form with a
   * warning.
   */
  public surfaceAuthError(message: string): void {
    this.clearFinishWatchdog();
    this.attemptId++;
    this.isHandoffStore.set(false);
    this.isFinishingStore.set(false);
    this.wasHiddenDuringHandoff = false;
    this.loginStateStore.set('idle');
    this.reportError(message);
  }

  /**
   * Register the visibility tracking that detects the user's return from the
   * external sign-in browser. Returns an unlisten the caller invokes on
   * teardown.
   */
  public attachVisibilityTracking(): () => void {
    document.addEventListener('visibilitychange', this.handleVisibilityChange);
    return () => document.removeEventListener('visibilitychange', this.handleVisibilityChange);
  }

  public openWiki(): Promise<void> {
    return HelpLinks.openWiki();
  }

  public openDiscord(): Promise<void> {
    return HelpLinks.openDiscord();
  }

  public openPrivacyNotice(): Promise<void> {
    return HelpLinks.openPrivacyNotice();
  }

  public teardownLoginFlow(): void {
    this.clearFinishWatchdog();
    this.stopHytalePolling();
  }

  private applyResult(result: LoginAttemptResult): void {
    switch (result.status) {
      case 'redirecting':
        // Auth URL opened in the system browser; hold the connecting view and
        // arm the return detector so we know when the user comes back.
        this.authUrlStore.set(result.authUrl ?? '');
        this.isHandoffStore.set(true);
        this.isFinishingStore.set(false);
        this.wasHiddenDuringHandoff = false;
        // The return detector is a visibility round-trip, and a phone does not reliably provide
        // one. This does not depend on being told anything.
        this.startPendingPoll();
        break;
      case 'error':
        this.attemptServerStore.set(result.sanitized);
        this.loginStateStore.set('error');
        break;
      case 'invalid':
        this.loginStateStore.set('idle');
        break;
      case 'navigating':
        break;
    }
  }

  private handleVisibilityChange = (): void => {
    if (get(this.loginStateStore) !== 'connecting' || !get(this.isHandoffStore)) return;
    if (document.visibilityState === 'hidden') {
      this.wasHiddenDuringHandoff = true;
      return;
    }
    if (document.visibilityState === 'visible' && this.wasHiddenDuringHandoff && !get(this.isFinishingStore)) {
      this.beginFinishing();
    }
  };

  private beginFinishing(): void {
    this.isFinishingStore.set(true);
    this.finishWatchdog.start();
  }

  /**
   * Whether an auth callback has arrived and not yet been disposed of.
   *
   * `pending_deep_link` is the one piece of state both halves maintain: Rust writes it for
   * every intent, and the handler clears it once the redemption has succeeded, failed, or
   * been recognised as a duplicate.
   */
  private async callbackInFlight(): Promise<boolean> {
    try {
      const store = await AppStore.load();
      return (await store.get<string>("pending_deep_link")) != null;
    } catch (e) {
      // An unreadable store is not evidence of progress; let the deadline stand.
      warn(`Login: could not check for a pending auth callback: ${e}`);
      return false;
    }
  }

  /**
   * Both ways of waiting for the browser, stopped together.
   *
   * Every caller that leaves the handoff — an error, a cancel, a fresh attempt, teardown —
   * already cancelled the watchdog here, so the poll belongs in the same place rather than in
   * five call sites that would each have to remember it.
   */
  private clearFinishWatchdog(): void {
    this.finishWatchdog.cancel();
    this.stopPendingPoll();
  }

  private startPendingPoll(): void {
    this.stopPendingPoll();
    this.pendingPoll = setInterval(() => {
      // Still the same attempt, and still waiting for the browser. Anything else means the poll
      // outlived what it was watching for.
      if (get(this.loginStateStore) !== 'connecting' || !get(this.isHandoffStore)) {
        this.stopPendingPoll();
        return;
      }
      void this.resumePendingDeepLink();
    }, Login.PENDING_POLL_MS);
  }

  private stopPendingPoll(): void {
    if (this.pendingPoll !== null) {
      clearInterval(this.pendingPoll);
      this.pendingPoll = null;
    }
  }

  async login(rawValue: string): Promise<LoginAttemptResult> {
    this.clearError();

    const trimmed = rawValue.trim();
    if (Login.isCodeLoginInput(trimmed)) {
      // The code flow is a screen the page switches to on `isCodeLogin`. Nothing to
      // navigate to, and no attempt to make here.
      return { status: 'navigating', sanitized: this.codeLoginServer(trimmed) };
    }

    const validation = this.validateServerUrl(rawValue);
    if (!validation.valid) {
      this.reportError(validation.error || I18n.t("Please enter a valid server URL"));
      return { status: 'invalid' };
    }

    const sanitizedUrl = this.sanitizeServerUrl(rawValue);
    info(sanitizedUrl + this.CONFIG_ENDPOINT);

    try {
      const response = await this.fetchWithTimeout(sanitizedUrl + this.CONFIG_ENDPOINT);

      if (response.status === 403) {
        warn("Server returned 403 Forbidden");
        return { status: 'error', sanitized: sanitizedUrl };
      }

      if (response.status !== 200) {
        throw new Error(`Bedrock Voice Chat Server ${sanitizedUrl} is not reachable.`);
      }

      info(`Successfully connected to Bedrock Voice Chat Server ${sanitizedUrl}`);
      const configData = await response.json();

      const clientId = configData.client_id;
      const secretState = self.crypto.randomUUID();

      const store = await AppStore.load();
      await store.set("auth_state_token", secretState);
      await store.set("auth_state_endpoint", sanitizedUrl);
      await store.save();

      const authLoginUrl = MinecraftAuthUrl.build(clientId, secretState);

      await this.openUrlWithLogging(authLoginUrl);
      return { status: 'redirecting', sanitized: sanitizedUrl, authUrl: authLoginUrl };
    } catch (e) {
      warn(String(e));
      return { status: 'error', sanitized: sanitizedUrl };
    }
  }

  private async openUrlWithLogging(url: string): Promise<void> {
    info(`Opening URL: ${url}`);
    await openUrl(url);
  }

  async loginWithHytale(rawValue: string): Promise<LoginAttemptResult> {
    this.clearError();

    const validation = this.validateServerUrl(rawValue);
    if (!validation.valid) {
      this.reportError(validation.error || I18n.t("Please enter a valid server URL"));
      return { status: 'invalid' };
    }

    const serverUrl = this.sanitizeServerUrl(rawValue);

    try {
      const configResponse = await this.fetchWithTimeout(serverUrl + this.CONFIG_ENDPOINT);

      if (configResponse.status === 403) {
        warn("Server returned 403 Forbidden");
        return { status: 'error', sanitized: serverUrl };
      }

      if (configResponse.status !== 200) {
        throw new Error(I18n.t("Server not reachable"));
      }

      const response = await invoke<HytaleDeviceFlowStartResponse>("start_hytale_device_flow", {
        server: serverUrl,
      });

      info(`Hytale Device flow started, session_id: ${response.session_id}, user_code: ${response.user_code}`);

      const store = await AppStore.load();
      await store.set("hytale_session_id", response.session_id);
      await store.set("auth_state_endpoint", serverUrl);
      await store.save();

      await this.openUrlWithLogging(response.verification_uri_complete);

      this.startHytalePolling(serverUrl, response.session_id, response.interval);
      return { status: 'redirecting', sanitized: serverUrl };
    } catch (e) {
      warn(`Hytale login failed: ${String(e)}`);
      return { status: 'error', sanitized: serverUrl };
    }
  }

  private startHytalePolling(server: string, sessionId: string, interval: number) {
    const pollInterval = Math.max(interval, 5) * 1000;
    info(`Starting Hytale polling with interval: ${pollInterval}ms`);

    this.hytalePollingInterval = window.setInterval(async () => {
      try {
        const response = await invoke<HytaleDeviceFlowStatusResponse>("poll_hytale_status", {
          server: server,
          sessionId: sessionId,
        });

        info(`Hytale poll response status: ${response.status}`);

        switch (response.status) {
          case "Pending":
            break;
          case "Success":
            this.stopHytalePolling();
            if (response.login_response) {
              await this.handleHytaleSuccess(server, response.login_response);
            }
            break;
          case "Expired":
            warn("Hytale device code expired");
            this.stopHytalePolling();
            this.reportError(I18n.t("Device code expired. Please try again."));
            break;
          case "Denied":
            warn("Hytale authorization denied");
            this.stopHytalePolling();
            this.reportError(I18n.t("Authorization denied. Please try again."));
            break;
          case "Error":
            error("Hytale auth error");
            this.stopHytalePolling();
            this.reportError(I18n.t("Authentication error. Please try again."));
            break;
        }
      } catch (e) {
        error(`Hytale polling error: ${String(e)}`);
        this.stopHytalePolling();
      }
    }, pollInterval);
  }

  private stopHytalePolling() {
    if (this.hytalePollingInterval !== null) {
      info("Stopping Hytale polling");
      window.clearInterval(this.hytalePollingInterval);
      this.hytalePollingInterval = null;
    }
  }

  private async handleHytaleSuccess(server: string, loginResponse: LoginResponse) {
    try {
      const store = await AppStore.load();

      await store.set("current_server", server);
      await store.set("current_player", loginResponse.gamertag);
      await store.set("active_game", "hytale");

      const serverList = await store.get("server_list") as ServerListEntry[] | null;
      const servers = serverList || [];

      const existing = servers.find(s => s.server === server);
      if (existing) {
        existing.game = "hytale";
      } else {
        servers.push({ server: server, player: loginResponse.gamertag, game: "hytale" });
      }
      await store.set("server_list", servers);
      ServerListStore.mirrorServerCount(servers);

      await store.delete("hytale_session_id");
      await store.save();

      Analytics.track("LoginCompleted", { game_type: "hytale" });
      window.location.href = "/setup";
    } catch (e) {
      error(`Failed to save login data: ${String(e)}`);
    }
  }
}
