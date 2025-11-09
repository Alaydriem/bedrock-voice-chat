import { onOpenUrl, getCurrent } from "@tauri-apps/plugin-deep-link";
import { info, error, warn, debug } from '@tauri-apps/plugin-log';
import { Store } from '@tauri-apps/plugin-store';
import { invoke } from "@tauri-apps/api/core";
import { platform } from '@tauri-apps/plugin-os';
import { type LoginResponse } from "../bindings/LoginResponse";
import Keyring from "./keyring";

/**
 * Centralized deep link manager
 * Handles OAuth callback deep links from any page in the app
 */
class DeepLinkManager {
    private initialized = false;
    private readonly AUTH_PATH = "://auth";
    private readonly PENDING_CALLBACK_KEY = "pending_auth_callback";

    /**
     * Initialize the deep link manager
     * Should be called as early as possible in app startup
     */
    async initialize(): Promise<void> {
        if (this.initialized) {
            return;
        }

        info("Initializing DeepLinkManager");

        // Register handler for deep links when app is already running
        await onOpenUrl(async (urls) => {
            info(`Deep link received (app running): ${urls.length} URL(s)`);
            for (const url of urls) {
                await this.handleDeepLink(url);
            }
        });

        // Handle deep links that cold-started the app (critical for Android OAuth flow)
        const currentUrls = await getCurrent();
        if (currentUrls && currentUrls.length > 0) {
            info(`Deep link received (cold start): ${currentUrls.length} URL(s)`);
            for (const url of currentUrls) {
                await this.handleDeepLink(url);
            }
        }

        this.initialized = true;
        info("DeepLinkManager initialized successfully");
    }

    /**
     * Get the platform-specific OAuth redirect URL
     */
    async getRedirectUrl(): Promise<string> {
        const store = await Store.load("store.json", {
            autoSave: false,
            defaults: {}
        });

        const redirectUrl = (() => {
            switch (platform()) {
                case "windows": return "bedrock-voice-chat://auth";
                case "android": return "bedrock-voice-chat://auth";
                case "ios": return "msauth.com.alaydriem.bvc.client://auth";
                default: throw new Error("Unsupported platform");
            }
        })();

        return redirectUrl;
    }

    /**
     * Handle a deep link URL
     */
    private async handleDeepLink(url: string): Promise<void> {
        info(`Processing deep link: ${url}`);

        const redirectUrl = await this.getRedirectUrl();

        // Check if this is an auth callback
        if (url.startsWith(redirectUrl)) {
            await this.handleAuthCallback(url);
        } else {
            info(`Deep link does not match auth pattern, ignoring: ${url}`);
        }
    }

    /**
     * Handle OAuth authentication callback
     */
    private async handleAuthCallback(url: string): Promise<void> {
        const parsedUrl = new URL(url);
        const code = parsedUrl.searchParams.get("code");
        const state = parsedUrl.searchParams.get("state");

        if (!code || !state) {
            error("Auth callback missing required parameters");
            return;
        }

        // Check if we're on the login page and ready to process
        const currentPath = window.location.pathname;
        if (currentPath === "/login") {
            // We're on the login page, process immediately
            await this.processAuthCallback(url, code, state);
        } else {
            // Store in persistent storage for later processing (survives page reloads)
            info(`Not on login page (current: ${currentPath}), storing callback for later`);
            const store = await Store.load("store.json", {
                autoSave: false,
                defaults: {}
            });
            await store.set(this.PENDING_CALLBACK_KEY, url);
            await store.save();

            // Navigate to login page if needed
            if (currentPath !== "/login") {
                info("Navigating to login page to process callback");
                window.location.href = "/login";
            }
        }
    }

    /**
     * Process a stored auth callback
     * Called by the login page when it's ready
     */
    async processPendingCallback(): Promise<boolean> {
        const store = await Store.load("store.json", {
            autoSave: false,
            defaults: {}
        });

        const url = await store.get<string>(this.PENDING_CALLBACK_KEY);
        if (!url) {
            return false;
        }

        // Clear the stored callback
        await store.delete(this.PENDING_CALLBACK_KEY);
        await store.save();

        const parsedUrl = new URL(url);
        const code = parsedUrl.searchParams.get("code");
        const state = parsedUrl.searchParams.get("state");

        if (!code || !state) {
            error("Stored callback missing required parameters");
            return false;
        }

        await this.processAuthCallback(url, code, state);
        return true;
    }

    /**
     * Process the OAuth callback with the server
     */
    private async processAuthCallback(url: string, code: string, state: string): Promise<void> {
        info(`Processing auth callback with code and state`);

        // Fetch the temporary variables from our store.json
        const store = await Store.load("store.json", {
            autoSave: false,
            defaults: {}
        });
        const authStateToken = await store.get<string>("auth_state_token");
        const authStateEndpoint = await store.get<string>("auth_state_endpoint");

        info(`Store values - authStateToken: ${authStateToken ? 'exists' : 'missing'}, authStateEndpoint: ${authStateEndpoint || 'missing'}`);

        // Verify that the state sent back from the server matches the one we generated
        if (state !== authStateToken) {
            error(`Auth State Mismatch - Expected: ${authStateToken}, Got: ${state}`);
            this.showLoginError();
            return;
        }

        // Invoke the server side ncryptf POST to the server and get the LoginResponse, or error
        const redirectUri = await this.getRedirectUrl();

        try {
            const response = await invoke("server_login", {
                code: code,
                server: authStateEndpoint,
                redirect: redirectUri
            }) as LoginResponse;

            info("Login successful, storing credentials");

            const keyring = await Keyring.new("servers");
            if (authStateEndpoint) {
                keyring.setServer(authStateEndpoint);

                // Insert and save data
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

                // Update server list
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

                info("Login complete, redirecting to dashboard");
                // Redirect to dashboard - the onboarding check will happen there
                window.location.href = "/dashboard";
            } else {
                throw new Error("authStateEndpoint is undefined");
            }
        } catch (e) {
            error(`Login failed: ${e}`);
            error(`Login error details: ${JSON.stringify(e)}`);
            this.showLoginError();
        }
    }

    /**
     * Show error on login form
     */
    private showLoginError(): void {
        const form = document.querySelector("#login-form");
        const serverUrl = form?.querySelector("#bvc-server-input");
        const errorMessage = form?.querySelector("#bvc-server-input-error-message");

        serverUrl?.classList.add("border-error");
        errorMessage?.classList.remove("invisible");
    }

    /**
     * Check if there's a pending auth callback
     */
    async hasPendingCallback(): Promise<boolean> {
        const store = await Store.load("store.json", {
            autoSave: false,
            defaults: {}
        });
        return await store.has(this.PENDING_CALLBACK_KEY);
    }
}

// Export singleton instance
export const deepLinkManager = new DeepLinkManager();
