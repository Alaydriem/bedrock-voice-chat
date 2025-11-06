import { fetch } from '@tauri-apps/plugin-http';
import { platform } from '@tauri-apps/plugin-os';
import { info, error, warn, debug } from '@tauri-apps/plugin-log';
import { Store } from '@tauri-apps/plugin-store';
import { openUrl } from '@tauri-apps/plugin-opener';
import { invoke } from "@tauri-apps/api/core";
import { onOpenUrl, getCurrent } from "@tauri-apps/plugin-deep-link";
import { type LoginResponse } from "../bindings/LoginResponse";
import Keyring from "./keyring.ts";
import App from './app.js';

declare global {
  interface Window {
    App: any;
    LoginDeepLinkRegistered: boolean;
  }
}

// Register any deeplink handlers on the splash page and they should retain registration
// This should only register _once_ on page load. Without this onOpenUrl _seems_ to get called
// multiple times (especially during dev mode) or page refresh
// This will only handle deep links for the ://auth endpoint.
// @todo: How do we _de-register_ this event handler?
if (!window.LoginDeepLinkRegistered) {
  // Handle deep links when app is already running
  await onOpenUrl(async (urls) => {
    info(`Deep link received (app running): ${urls.length} URL(s)`);
    for (const url of urls) {
      info(`Processing deep link: ${url}`);
      if (url.startsWith(await Login.getRedirectUrl())) {
        await Login.openDeepLink(url);
      }
    }
  });

  // Handle deep links that cold-started the app (critical for Android OAuth flow)
  const currentUrls = await getCurrent();
  if (currentUrls && currentUrls.length > 0) {
    info(`Deep link received (cold start): ${currentUrls.length} URL(s)`);
    for (const url of currentUrls) {
      info(`Processing cold start deep link: ${url}`);
      if (url.startsWith(await Login.getRedirectUrl())) {
        await Login.openDeepLink(url);
      }
    }
  }

  window.LoginDeepLinkRegistered = true;
}

export default class Login extends App {
  // This is the endpoint that the API configuration can be retrieved from
  readonly CONFIG_ENDPOINT= "/api/config";
  // This is the POST endpoint for authenticating with the server
  readonly AUTH_ENDPOINT = "/api/auth";
  // This is the GET endpoint for getting a fresh ncryptf key
  readonly NCRYPTF_EK_ENDPOINT = "/ncryptf/ek";

  constructor() {
    super();
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
      const redirectUrl = await Login.getRedirectUrl();
      const authLoginUrl: string =
        `https://login.live.com/oauth20_authorize.srf?client_id=${clientId}&response_type=code&redirect_uri=${redirectUrl}&scope=XboxLive.signin%20offline_access&state=${secretState}`;

      openUrl(authLoginUrl);
    }).catch((e) => {
      warn(String(e));
      serverUrl.classList.add("border-error");
      errorMessage.classList.remove("invisible");
    });
  }

  // This will return all the "correct" OAuth2 redirect URLs that are platform specific
  static async getRedirectUrl() {
    const store = await Store.load("store.json", {
        autoSave: false,
        defaults: {}
    });
    const androidSignatureHash = await store.get("android_signature_hash");

    const redirectUrl = (() => {
      switch (platform()) {
        case "windows": return "bedrock-voice-chat://auth";
        case "android": return "bedrock-voice-chat://auth";
        case "ios": return "msauth.com.alaydriem.bvc.client://auth";
        default: throw new Error("Unsupported platform");
      };
    })();

    return redirectUrl;
  }

  // This is our event handler for the ://auth deep link event
  static async openDeepLink(url: string) {
    info(`openDeepLink called with URL: ${url}`);

    // Fetch the temporary variables from our store.json
    const store = await Store.load("store.json", {
        autoSave: false,
        defaults: {}
    });
    const authStateToken = await store.get<string>("auth_state_token");
    const authStateEndpoint = await store.get<string>("auth_state_endpoint");

    info(`Store values - authStateToken: ${authStateToken ? 'exists' : 'missing'}, authStateEndpoint: ${authStateEndpoint || 'missing'}`);

    const code = new URL(url).searchParams.get("code");
    const state = new URL(url).searchParams.get("state");

    info(`URL params - code: ${code ? 'exists' : 'missing'}, state: ${state || 'missing'}`);

    // Verify that the state sent back from the server matches the one we generated
    if (state !== authStateToken) {
      error(`Auth State Mismatch - Expected: ${authStateToken}, Got: ${state}`);

      // Try to update UI if DOM is ready, but don't fail silently
      const form = document.querySelector("#login-form");
      const serverUrl = form?.querySelector("#bvc-server-input");
      const errorMessage = form?.querySelector("#bvc-server-input-error-message");

      serverUrl?.classList.add("border-error");
      errorMessage?.classList.remove("invisible");
      return;
    }

    // Invoke the server side ncryptf POST to the server and get the LoginResponse, or error
    const redirectUri = await Login.getRedirectUrl();
    await invoke("server_login", {
      code: code,
      server: authStateEndpoint,
      redirect: redirectUri
    })
    .then(async (response) => response as LoginResponse)
    .then(async(response) => {
      const keyring = await Keyring.new("servers");
      if (authStateEndpoint) {
        keyring.setServer(authStateEndpoint);
        // Insert and save data, commit, set the current server, then redirect to the dashboard
        Object.keys(response).forEach(async key => {
          const value = response[key as keyof LoginResponse];
          if (typeof value === "string" || value instanceof Uint8Array) {
            await keyring.insert(key, value);
          } else {
            await keyring.insert(key, JSON.stringify(value));
          }
        });
        await store.set("current_server", authStateEndpoint);
        await store.set("current_player", response.gamertag);
        if (await store.has("server_list")) {
          let serverList = await store.get("server_list") as Array<{ server: string, player: string }>;
          let hasServer = false;
          serverList.forEach(server => {
            if (server.server == authStateEndpoint) {
              hasServer = true;
            }
          });

          if (!hasServer) {
            serverList.push({
              "server": authStateEndpoint,
              "player": response.gamertag
            });
            await store.set("server_list", serverList);
          }
        } else {
          let serverList = [];
          serverList.push({
            "server": authStateEndpoint,
            "player": response.gamertag
          });
          await store.set("server_list", serverList);
        }

        // Clean up temporary auth tokens after successful login
        await store.delete("auth_state_token");
        await store.delete("auth_state_endpoint");
        await store.save();

        window.location.href = "/dashboard";
      } else {
        throw new Error("authStateEndpoint is undefined");
      }
    }).catch((e) => {
      error(`Login failed: ${e}`);
      error(`Login error details: ${JSON.stringify(e)}`);

      // Try to update UI if DOM is ready, but don't fail silently
      const form = document.querySelector("#login-form");
      const serverUrl = form?.querySelector("#bvc-server-input");
      const errorMessage = form?.querySelector("#bvc-server-input-error-message");

      serverUrl?.classList.add("border-error");
      errorMessage?.classList.remove("invisible");
    });

    // Ensure we only handle a single deep link event
    return;
  }
}
