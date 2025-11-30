import { fetch } from '@tauri-apps/plugin-http';
import { info, error, warn, debug } from '@tauri-apps/plugin-log';
import { Store } from '@tauri-apps/plugin-store';
import { openUrl } from '@tauri-apps/plugin-opener';
import { platform } from '@tauri-apps/plugin-os';
import BVCApp from './BVCApp.ts';

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

  constructor() {
    super();
  }

  // Initialize login page and check for pending deep link callbacks
  async initialize() {
    await this.initializeDeepLinks();
  }

  // Sanitize server URL to ensure https:// prefix and no trailing slash
  private sanitizeServerUrl(url: string): string {
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

  // This is the main event handler for the form submission
  async login(event: any) {
    let form = event.currentTarget;
    // Setup some variables
    const serverUrl = form.querySelector("#bvc-server-input");
    const errorMessage = form.querySelector("#bvc-server-input-error-message");
    serverUrl.classList.remove("border-error");
    errorMessage.classList.add("invisible");

    // Sanitize the server URL
    const sanitizedUrl = this.sanitizeServerUrl(serverUrl.value);

    // Update the input field with sanitized value
    serverUrl.value = sanitizedUrl;

    info(sanitizedUrl + this.CONFIG_ENDPOINT);
    // Fetch the configuration from the server and retrieve the client_id for
    // Authenticating with Microsoft
    await fetch(sanitizedUrl + this.CONFIG_ENDPOINT, {
      method: 'GET'
    }).then(async (response) => {
      if (response.status !== 200) {
        throw new Error("Bedrock Voice Chat Server " + sanitizedUrl + " is not reachable.");
      }
      info("Successfully connected to Bedrock Voice Chat Server " + sanitizedUrl);
      return await response.json();
    }).then(async (response) => {
      const clientId = response.client_id;
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

      openUrl(authLoginUrl);
    }).catch((e) => {
      warn(String(e));
      serverUrl.classList.add("border-error");
      errorMessage.classList.remove("invisible");
    });
  }

  private getRedirectUrl(): string {
    return 'bedrock-voice-chat://auth';
  }
}
