import { fetch } from '@tauri-apps/plugin-http';
import { info, error, warn, debug } from '@tauri-apps/plugin-log';
import { Store } from '@tauri-apps/plugin-store';
import { openUrl } from '@tauri-apps/plugin-opener';
import { invoke } from '@tauri-apps/api/core';
import BVCApp from './BVCApp.ts';
import Keyring from './keyring.ts';
import type { HytaleDeviceFlowStartResponse, HytaleDeviceFlowStatusResponse, HytaleAuthStatus, LoginResponse } from '../bindings/index.ts';

declare global {
  interface Window {
    App: any;
  }
}

export default class Login extends BVCApp {
  // This is the endpoint that the API configuration can be retrieved from
  readonly CONFIG_ENDPOINT= "/api/config";
  // This is the POST endpoint for authenticating with the server
  readonly AUTH_ENDPOINT = "/api/auth";
  // This is the GET endpoint for getting a fresh ncryptf key
  readonly NCRYPTF_EK_ENDPOINT = "/ncryptf/ek";

  // Hytale polling state
  private hytalePollingInterval: number | null = null;

  constructor() {
    super();
  }

  // Initialize login page and check for pending deep link callbacks
  async initialize() {
    await this.initializeDeepLinks();
  }

  // Sanitize server URL to ensure https:// prefix and no trailing slash
  public sanitizeServerUrl(url: string): string {
    let sanitized = url.trim();

    // Ensure https:// prefix
    if (!sanitized.startsWith("https://")) {
      if (sanitized.startsWith("http://")) {
        // Replace http:// with https://
        sanitized = sanitized.replace("http://", "https://");
      } else {
        // Add https:// prefix
        sanitized = "https://" + sanitized;
      }
    }

    // Remove trailing slash
    if (sanitized.endsWith("/")) {
      sanitized = sanitized.slice(0, -1);
    }

    return sanitized;
  }

  // Validate server URL format
  public validateServerUrl(url: string): { valid: boolean; error?: string } {
    const trimmed = url.trim();

    if (!trimmed) {
      return { valid: false, error: "Please enter a server URL" };
    }

    // Basic URL pattern validation (after sanitization would add https://)
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

  // Set error message on the login form
  private setErrorMessage(errorElement: Element | null, message: string) {
    if (errorElement instanceof HTMLElement) {
      errorElement.innerText = message;
      errorElement.classList.remove("invisible");
    }
  }

  // This is the main event handler for the form submission
  async login(event: any) {
    event.preventDefault();

    // Setup some variables
    const serverInput = document.querySelector("#bvc-server-input") as HTMLInputElement;
    const errorMessage = document.querySelector("#bvc-server-input-error-message");
    serverInput?.classList.remove("border-error");
    errorMessage?.classList.add("invisible");

    // Validate the server URL first
    const validation = this.validateServerUrl(serverInput?.value || "");
    if (!validation.valid) {
      serverInput?.classList.add("border-error");
      this.setErrorMessage(errorMessage, validation.error || "Please enter a valid server URL");
      return;
    }

    // Sanitize the server URL
    const sanitizedUrl = this.sanitizeServerUrl(serverInput?.value || "");

    // Update the input field with sanitized value
    if (serverInput) {
      serverInput.value = sanitizedUrl;
    }

    info(sanitizedUrl + this.CONFIG_ENDPOINT);

    try {
      // Fetch the configuration from the server and retrieve the client_id for
      // Authenticating with Microsoft
      const response = await fetch(sanitizedUrl + this.CONFIG_ENDPOINT, {
        signal: AbortSignal.timeout(5000),
        method: 'GET'
      });

      if (response.status === 403) {
        warn("Server returned 403 Forbidden");
        serverInput?.classList.add("border-error");
        this.setErrorMessage(errorMessage, "Access denied. Check with your server operator if you have permissions.");
        return;
      }

      if (response.status !== 200) {
        throw new Error("Bedrock Voice Chat Server " + sanitizedUrl + " is not reachable.");
      }

      info("Successfully connected to Bedrock Voice Chat Server " + sanitizedUrl);
      const configData = await response.json();

      const clientId = configData.client_id;
      const secretState = self.crypto.randomUUID();
      // Store some temporary tokens during the login phase.
      const store = await Store.load("store.json", {
          autoSave: false,
          defaults: {}
      });
      await store.set("auth_state_token", secretState);
      await store.set("auth_state_endpoint", sanitizedUrl);
      await store.save();

      // Open a browser Window to authenticate with Microsoft Services
      const redirectUrl = this.getRedirectUrl();
      const authLoginUrl: string =
        `https://login.live.com/oauth20_authorize.srf?client_id=${clientId}&response_type=code&redirect_uri=${redirectUrl}&scope=XboxLive.signin%20offline_access&state=${secretState}`;

      await this.openUrlWithLogging(authLoginUrl);
    } catch (e) {
      warn(String(e));
      serverInput?.classList.add("border-error");
      this.setErrorMessage(errorMessage, "Cannot connect to Bedrock Voice Chat server. Confirm the URL and access permissions with your server operator.");
    }
  }

  private getRedirectUrl(): string {
    return 'bedrock-voice-chat://auth';
  }

  // Helper method to open URLs with logging
  private async openUrlWithLogging(url: string): Promise<void> {
    info(`Opening URL: ${url}`);
    await openUrl(url);
  }

  // Start Hytale device flow authentication
  async loginWithHytale(event: any) {
    event.preventDefault();

    const serverInput = document.querySelector("#bvc-server-input") as HTMLInputElement;
    const errorMessage = document.querySelector("#bvc-server-input-error-message");

    // Clear any previous errors
    serverInput?.classList.remove("border-error");
    errorMessage?.classList.add("invisible");

    // Validate the server URL first
    const validation = this.validateServerUrl(serverInput?.value || "");
    if (!validation.valid) {
      serverInput?.classList.add("border-error");
      this.setErrorMessage(errorMessage, validation.error || "Please enter a valid server URL");
      return;
    }

    // Sanitize the server URL
    const serverUrl = this.sanitizeServerUrl(serverInput?.value || "");

    // Update the input field with sanitized value
    if (serverInput) {
      serverInput.value = serverUrl;
    }

    try {
      // First verify the server is reachable
      const configResponse = await fetch(serverUrl + this.CONFIG_ENDPOINT, {
        signal: AbortSignal.timeout(5000),
        method: 'GET'
      });

      if (configResponse.status === 403) {
        warn("Server returned 403 Forbidden");
        serverInput?.classList.add("border-error");
        this.setErrorMessage(errorMessage, "Access denied. Check with your server operator if you have permissions.");
        return;
      }

      if (configResponse.status !== 200) {
        throw new Error("Server not reachable");
      }

      // Start the device flow via Tauri command
      const response = await invoke<HytaleDeviceFlowStartResponse>("start_hytale_device_flow", {
        server: serverUrl
      });

      info(`Hytale Device flow started, session_id: ${response.session_id}, user_code: ${response.user_code}`);

      // Store state for polling
      const store = await Store.load("store.json", {
        autoSave: false,
        defaults: {}
      });
      await store.set("hytale_session_id", response.session_id);
      await store.set("auth_state_endpoint", serverUrl);
      await store.save();

      // Open verification URL immediately (with pre-filled code)
      await this.openUrlWithLogging(response.verification_uri_complete);

      // Start polling in background
      this.startHytalePolling(serverUrl, response.session_id, response.interval);
    } catch (e) {
      warn(`Hytale login failed: ${String(e)}`);
      serverInput?.classList.add("border-error");
      this.setErrorMessage(errorMessage, "Cannot connect to Bedrock Voice Chat server. Confirm the URL and access permissions with your server operator.");
    }
  }

  // Start polling for Hytale device flow status
  private startHytalePolling(server: string, sessionId: string, interval: number) {
    // Use at least 5 seconds as the interval
    const pollInterval = Math.max(interval, 5) * 1000;

    info(`Starting Hytale polling with interval: ${pollInterval}ms`);

    this.hytalePollingInterval = window.setInterval(async () => {
      try {
        const response = await invoke<HytaleDeviceFlowStatusResponse>("poll_hytale_status", {
          server: server,
          sessionId: sessionId
        });

        info(`Hytale poll response status: ${response.status}`);

        switch (response.status) {
          case "Pending":
            // Continue polling
            break;
          case "Success":
            // Stop polling and handle success
            this.stopHytalePolling();
            if (response.login_response) {
              await this.handleHytaleSuccess(server, response.login_response);
            }
            break;
          case "Expired":
            warn("Hytale device code expired");
            this.stopHytalePolling();
            // Show error to user
            const errorMessage = document.querySelector("#bvc-server-input-error-message");
            errorMessage?.classList.remove("invisible");
            if (errorMessage) {
              errorMessage.textContent = "Device code expired. Please try again.";
            }
            break;
          case "Denied":
            warn("Hytale authorization denied");
            this.stopHytalePolling();
            const deniedError = document.querySelector("#bvc-server-input-error-message");
            deniedError?.classList.remove("invisible");
            if (deniedError) {
              deniedError.textContent = "Authorization denied. Please try again.";
            }
            break;
          case "Error":
            error("Hytale auth error");
            this.stopHytalePolling();
            const authError = document.querySelector("#bvc-server-input-error-message");
            authError?.classList.remove("invisible");
            if (authError) {
              authError.textContent = "Authentication error. Please try again.";
            }
            break;
        }
      } catch (e) {
        error(`Hytale polling error: ${String(e)}`);
        this.stopHytalePolling();
      }
    }, pollInterval);
  }

  // Stop Hytale polling
  private stopHytalePolling() {
    if (this.hytalePollingInterval !== null) {
      info("Stopping Hytale polling");
      window.clearInterval(this.hytalePollingInterval);
      this.hytalePollingInterval = null;
    }
  }

  // Handle successful Hytale authentication
  private async handleHytaleSuccess(server: string, loginResponse: LoginResponse) {
    try {
      const store = await Store.load("store.json", {
        autoSave: false,
        defaults: {}
      });

      // Store credentials to keyring (same pattern as Minecraft auth)
      const keyring = await Keyring.new("servers");
      await keyring.setServer(server);

      for (const key of Object.keys(loginResponse)) {
        const value = loginResponse[key as keyof LoginResponse];
        if (value === null || value === undefined) {
          continue;
        }
        if (typeof value === "string" || value instanceof Uint8Array) {
          await keyring.insert(key, value);
        } else {
          await keyring.insert(key, JSON.stringify(value));
        }
      }

      // Store current server and player info
      await store.set("current_server", server);
      await store.set("current_player", loginResponse.gamertag);
      await store.set("active_game", "hytale");

      // Add to server list if not already present
      const serverList = await store.get("server_list") as Array<{ server: string, player: string }> | null;
      const servers = serverList || [];

      if (!servers.some(s => s.server === server)) {
        servers.push({ server: server, player: loginResponse.gamertag });
        await store.set("server_list", servers);
      }

      // Clean up Hytale session data
      await store.delete("hytale_session_id");
      await store.save();

      window.location.href = "/onboarding/welcome";
    } catch (e) {
      error(`Failed to save login data: ${String(e)}`);
    }
  }
}
