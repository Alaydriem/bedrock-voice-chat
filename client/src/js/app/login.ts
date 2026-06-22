import { fetch } from '@tauri-apps/plugin-http';
import { info, error, warn } from '@tauri-apps/plugin-log';
import { Store } from '@tauri-apps/plugin-store';
import { openUrl } from '@tauri-apps/plugin-opener';
import { getVersion } from '@tauri-apps/api/app';
import { invoke } from '@tauri-apps/api/core';
import { writable, type Writable, type Readable } from 'svelte/store';
import BVCApp from './BVCApp.ts';
import Analytics from './analytics';
import PlatformDetector from './utils/PlatformDetector.ts';
import type { HytaleDeviceFlowStartResponse, HytaleDeviceFlowStatusResponse, LoginResponse, ServerListEntry } from '../bindings/index.ts';
import type { LoginPageState } from './login/LoginPageState';

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
  | { status: 'redirecting'; sanitized: string }
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

  public readonly isMobileReadable: Readable<boolean>;
  public readonly appVersionReadable: Readable<string>;
  public readonly formError: Readable<string>;
  public readonly serverInputInvalid: Readable<boolean>;

  private platformDetector: PlatformDetector;

  constructor() {
    super();
    this.isMobileStore = writable(false);
    this.appVersionStore = writable('');
    this.formErrorStore = writable('');
    this.serverInputInvalidStore = writable(false);
    this.isMobileReadable = { subscribe: this.isMobileStore.subscribe };
    this.appVersionReadable = { subscribe: this.appVersionStore.subscribe };
    this.formError = { subscribe: this.formErrorStore.subscribe };
    this.serverInputInvalid = { subscribe: this.serverInputInvalidStore.subscribe };
    this.platformDetector = new PlatformDetector();
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

    await invoke('reset_asm').catch(() => {});
    await invoke('reset_nsm').catch(() => {});

    await this.initialize();
    this.preloader();

    const urlParams = new URLSearchParams(window.location.search);
    const isAddServer = urlParams.has('addserver');

    let backHref = '/dashboard';
    let backLabel = 'Back to Dashboard';
    if (isAddServer) {
      const returnTarget = urlParams.get('return') ?? '';
      if (returnTarget === '/server') {
        backHref = '/server';
        backLabel = 'Back to Server List';
      }
    }

    const prefilledServer = urlParams.get('server') ?? '';
    const autoReauth = urlParams.has('server') && urlParams.get('reauth') === 'true';

    return { isAddServer, backHref, backLabel, prefilledServer, autoReauth };
  }

  handleCodeLoginNavigate(rawValue: string): string {
    const server = rawValue.replace(/^(https?:\/\/)?code@/, (_, proto) => proto ?? 'https://');
    return `/login/code?server=${encodeURIComponent(server)}`;
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
      return { valid: false, error: "Please enter a server URL" };
    }

    const sanitized = this.sanitizeServerUrl(trimmed);
    try {
      const parsed = new URL(sanitized);
      if (parsed.protocol !== "https:") {
        return { valid: false, error: "Server URL must use HTTPS" };
      }
      if (!parsed.hostname || parsed.hostname.length < 3) {
        return { valid: false, error: "Please enter a valid server URL" };
      }
    } catch {
      return { valid: false, error: "Please enter a valid server URL" };
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
  }

  /**
   * Cancel an in-progress Hytale device-flow poll. Used when the user backs
   * out of the "connecting" handoff view so we don't keep polling a flow they
   * abandoned.
   */
  public cancelHytalePolling(): void {
    this.stopHytalePolling();
  }

  async login(rawValue: string): Promise<LoginAttemptResult> {
    this.clearError();

    const trimmed = rawValue.trim();
    if (Login.isCodeLoginInput(trimmed)) {
      const serverPart = trimmed.replace(/^(https?:\/\/)?code@/, '');
      const sanitized = this.sanitizeServerUrl(serverPart);
      window.location.href = `/login/code?server=${encodeURIComponent(sanitized)}`;
      return { status: 'navigating', sanitized };
    }

    const validation = this.validateServerUrl(rawValue);
    if (!validation.valid) {
      this.reportError(validation.error || "Please enter a valid server URL");
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

      const store = await Store.load("store.json", { autoSave: false, defaults: {} });
      await store.set("auth_state_token", secretState);
      await store.set("auth_state_endpoint", sanitizedUrl);
      await store.save();

      const redirectUrl = this.getRedirectUrl();
      const authLoginUrl =
        `https://login.live.com/oauth20_authorize.srf?client_id=${clientId}&response_type=code&redirect_uri=${redirectUrl}&scope=XboxLive.signin%20offline_access&state=${secretState}`;

      await this.openUrlWithLogging(authLoginUrl);
      return { status: 'redirecting', sanitized: sanitizedUrl };
    } catch (e) {
      warn(String(e));
      return { status: 'error', sanitized: sanitizedUrl };
    }
  }

  private getRedirectUrl(): string {
    return 'bedrock-voice-chat://auth';
  }

  private async openUrlWithLogging(url: string): Promise<void> {
    info(`Opening URL: ${url}`);
    await openUrl(url);
  }

  async loginWithHytale(rawValue: string): Promise<LoginAttemptResult> {
    this.clearError();

    const validation = this.validateServerUrl(rawValue);
    if (!validation.valid) {
      this.reportError(validation.error || "Please enter a valid server URL");
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
        throw new Error("Server not reachable");
      }

      const response = await invoke<HytaleDeviceFlowStartResponse>("start_hytale_device_flow", {
        server: serverUrl,
      });

      info(`Hytale Device flow started, session_id: ${response.session_id}, user_code: ${response.user_code}`);

      const store = await Store.load("store.json", { autoSave: false, defaults: {} });
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
            this.reportError("Device code expired. Please try again.");
            break;
          case "Denied":
            warn("Hytale authorization denied");
            this.stopHytalePolling();
            this.reportError("Authorization denied. Please try again.");
            break;
          case "Error":
            error("Hytale auth error");
            this.stopHytalePolling();
            this.reportError("Authentication error. Please try again.");
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
      const store = await Store.load("store.json", { autoSave: false, defaults: {} });

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

      await store.delete("hytale_session_id");
      await store.save();

      Analytics.track("LoginCompleted", { game_type: "hytale" });
      window.location.href = "/onboarding/welcome";
    } catch (e) {
      error(`Failed to save login data: ${String(e)}`);
    }
  }
}
