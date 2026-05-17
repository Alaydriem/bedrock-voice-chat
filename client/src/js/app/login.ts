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

export default class Login extends BVCApp {
  private static readonly CODE_LOGIN_PATTERN = /^(https?:\/\/)?code@/;

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

  private clearError(): void {
    this.formErrorStore.set('');
    this.serverInputInvalidStore.set(false);
  }

  private reportError(message: string): void {
    this.formErrorStore.set(message);
    this.serverInputInvalidStore.set(true);
  }

  async login(rawValue: string): Promise<{ sanitized?: string }> {
    this.clearError();

    const trimmed = rawValue.trim();
    if (Login.isCodeLoginInput(trimmed)) {
      const serverPart = trimmed.replace(/^(https?:\/\/)?code@/, '');
      const sanitized = this.sanitizeServerUrl(serverPart);
      window.location.href = `/login/code?server=${encodeURIComponent(sanitized)}`;
      return { sanitized };
    }

    const validation = this.validateServerUrl(rawValue);
    if (!validation.valid) {
      this.reportError(validation.error || "Please enter a valid server URL");
      return {};
    }

    const sanitizedUrl = this.sanitizeServerUrl(rawValue);
    info(sanitizedUrl + this.CONFIG_ENDPOINT);

    try {
      const response = await fetch(sanitizedUrl + this.CONFIG_ENDPOINT, {
        signal: AbortSignal.timeout(5000),
        method: 'GET',
      });

      if (response.status === 403) {
        warn("Server returned 403 Forbidden");
        this.reportError("Access denied. Check with your server operator if you have permissions.");
        return { sanitized: sanitizedUrl };
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
      return { sanitized: sanitizedUrl };
    } catch (e) {
      warn(String(e));
      this.reportError("Cannot connect to Bedrock Voice Chat server. Confirm the URL and access permissions with your server operator.");
      return { sanitized: sanitizedUrl };
    }
  }

  private getRedirectUrl(): string {
    return 'bedrock-voice-chat://auth';
  }

  private async openUrlWithLogging(url: string): Promise<void> {
    info(`Opening URL: ${url}`);
    await openUrl(url);
  }

  async loginWithHytale(rawValue: string): Promise<{ sanitized?: string }> {
    this.clearError();

    const validation = this.validateServerUrl(rawValue);
    if (!validation.valid) {
      this.reportError(validation.error || "Please enter a valid server URL");
      return {};
    }

    const serverUrl = this.sanitizeServerUrl(rawValue);

    try {
      const configResponse = await fetch(serverUrl + this.CONFIG_ENDPOINT, {
        signal: AbortSignal.timeout(5000),
        method: 'GET',
      });

      if (configResponse.status === 403) {
        warn("Server returned 403 Forbidden");
        this.reportError("Access denied. Check with your server operator if you have permissions.");
        return { sanitized: serverUrl };
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
      return { sanitized: serverUrl };
    } catch (e) {
      warn(`Hytale login failed: ${String(e)}`);
      this.reportError("Cannot connect to Bedrock Voice Chat server. Confirm the URL and access permissions with your server operator.");
      return { sanitized: serverUrl };
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
