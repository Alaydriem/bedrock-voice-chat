import { I18n } from "$lib/i18n";
import { Store } from '@tauri-apps/plugin-store';
import { invoke } from "@tauri-apps/api/core";
import { info, error as logError } from '@charlesportwoodii/tauri-plugin-curia';
import Analytics from '../analytics';
import { type LoginResponse } from "../../bindings/LoginResponse";
import { type ServerListEntry } from "../../bindings/ServerListEntry";
import { ServerListStore } from '../services/ServerListStore';
import MinecraftRedirect from '../auth/MinecraftRedirect';
import type { DeepLinkOutcome } from '../deepLinkRouter.ts';

export class AuthCallbackHandler {
    private readonly AUTH_PREFIXES = [
        'bedrock-voice-chat://auth',
        'https://bvc.alaydriem.com/auth',
        'https://www.bedrockvoicechat.com/auth'
    ];
    // Must match Login.LOGIN_ERROR_KEY. Duplicated rather than imported to avoid
    // a Login -> BVCApp -> deepLinkRouter -> authCallbackHandler -> Login cycle.
    private readonly LOGIN_ERROR_KEY = "login_error";
    // Must match DeepLinkRouter.PENDING_KEY. Cleared here, before navigating, so
    // the single-use callback is never re-redeemed on the next page mount.
    private readonly PENDING_KEY = "pending_deep_link";
    /**
     * Fingerprints of authorization codes already sent for exchange, most recent last.
     *
     * `DeepLinkRouter.routedUrls` answers the same question in memory and cannot answer it
     * across a page load. Every deep-link intent both writes `pending_deep_link` and emits
     * `deep-link-received`, and on Android an intent arrives while the app is backgrounded
     * or cold — so the callback routinely lands either side of a navigation, and the new
     * context starts with an empty set and a pending entry the old one had not yet cleared.
     * The second exchange spends a code the provider has already retired, which comes back
     * as an invalid code and reads on screen as the account being refused.
     */
    private readonly REDEEMED_KEY = "redeemed_auth_codes";
    /** Enough to cover a retried sign-in; this is a duplicate guard, not an audit log. */
    private readonly REDEEMED_LIMIT = 5;
    private store: Store;

    constructor(store: Store) {
        this.store = store;
    }

    canHandle(url: string): boolean {
        return this.AUTH_PREFIXES.some(prefix => url.startsWith(prefix));
    }

    async handle(url: string): Promise<DeepLinkOutcome> {
        info(`AuthCallbackHandler: Processing auth callback`);

        const parsedUrl = new URL(url);
        const code = parsedUrl.searchParams.get("code");
        const state = parsedUrl.searchParams.get("state");

        if (!code || !state) {
            throw new Error(I18n.t("Auth callback missing required parameters"));
        }

        // Redemption has no page dependency: redeem wherever the link arrives and
        // navigate based on the outcome (onboarding on success, /login on
        // retryable failure, /error on access denial). This avoids stranding the
        // callback when the landing page (splash, server list) navigates away
        // before a deferred /login redirect can take effect.
        await this.processAuthCallback(code, state);
        return 'handled';
    }

    private async processAuthCallback(code: string, state: string): Promise<void> {
        /*
         * Before the state comparison, because a successful login deletes the state token.
         * A duplicate arriving after that would fail the comparison instead of being
         * recognised as the copy it is, and failLogin would take someone who is already
         * signed in and part-way through setup back to the login page under an
         * authentication error.
         */
        if (await this.alreadyRedeemed(code)) {
            info(I18n.t("AuthCallbackHandler: Authorization code already exchanged"));
            await this.store.delete(this.PENDING_KEY);
            await this.store.save();
            // A spent code is not evidence the sign-in finished, and this branch used to assume
            // it was — it returned, navigating nowhere and reporting nothing.
            //
            // The code is claimed *before* the exchange, deliberately, so a code sent once is
            // spent whatever happened next. On Android the auth intent routinely reloads the
            // webview mid-exchange: the in-flight `server_login` dies with the context, the
            // pending callback survives because only success clears it, and the fresh context
            // routes it again and lands exactly here. Silence then looked like a login that did
            // nothing — no error, still on the sign-in page, or still holding "continue in the
            // window that just opened" — which is what the user was actually seeing.
            //
            // So ask whether the session exists instead of inferring it from the code.
            await this.resumeOrFail();
            return;
        }

        const authStateToken = await this.store.get<string>("auth_state_token");
        const authStateEndpoint = await this.store.get<string>("auth_state_endpoint");

        if (state !== authStateToken) {
            logError(`AuthCallbackHandler: Auth state mismatch - Expected: ${authStateToken}, Got: ${state}`);
            await this.failLogin(I18n.t("Authentication failed. Please try again."));
            return;
        }

        if (!authStateEndpoint) {
            logError("AuthCallbackHandler: auth_state_endpoint is undefined");
            await this.failLogin(I18n.t("Login failed. Please check your server URL and try again."));
            return;
        }

        await this.claimCode(code);

        const redirectUri = this.getRedirectUrl();

        try {
            const response = await invoke("server_login", {
                code: code,
                server: authStateEndpoint,
                redirect: redirectUri
            }) as LoginResponse;

            await this.establishSession(authStateEndpoint, response);
        } catch (e) {
            logError(`AuthCallbackHandler: Login failed: ${e}`);
            const errorStr = String(e).toLowerCase();
            // Ahead of the 403 chain: a platform storage message can contain "denied", and
            // classified as an access denial it would tell someone to join the Minecraft server
            // in-game to fix their keyring.
            if (errorStr.includes("auth04")) {
                await this.failStorage("AUTH04");
            } else if (errorStr.includes("auth03")) {
                await this.failStorage("AUTH03");
            } else if (errorStr.includes("403") || errorStr.includes("forbidden") || errorStr.includes("denied") || errorStr.includes("banned") || errorStr.includes("whitelist")) {
                await this.denyLogin();
            } else if (errorStr.includes("401")) {
                // The sign-in did not complete. Sending someone to check their server URL
                // for this points them at the one thing that was working.
                await this.failLogin(I18n.t("That sign-in could not be completed. Please sign in again."));
            } else if (errorStr.includes("502")) {
                await this.failLogin(I18n.t("Xbox Live could not be reached. Please try again in a moment."));
            } else {
                await this.failLogin(I18n.t("Login failed. Please check your server URL and try again."));
            }
        }
    }

    private getRedirectUrl(): string {
        return MinecraftRedirect.URI;
    }

    /**
     * Record a completed sign-in and go on to setup.
     *
     * Shared with the resume path rather than duplicated, because the two must agree about what
     * being signed in consists of. `server_login` writes the credentials from Rust; everything
     * here is the webview-side bookkeeping the rest of the app reads, and a resume that restored
     * only some of it would produce a session that authenticates and then cannot find itself.
     *
     * `/setup`, not `/dashboard`: setup is re-checked on every launch and forwards to the
     * dashboard by itself, so this cannot skip a permission prompt nobody has answered.
     */
    private async establishSession(endpoint: string, response: LoginResponse): Promise<void> {
        await this.store.set("current_server", endpoint);
        await this.store.set("current_player", response.gamertag);
        await this.store.set("active_game", "minecraft");

        const existing = ((await this.store.get<ServerListEntry[]>("server_list")) ?? []).map(
            (server) =>
                server.server === endpoint ? { ...server, game: "minecraft" as const } : server,
        );
        const serverList = existing.some((server) => server.server === endpoint)
            ? existing
            : [
                  ...existing,
                  { server: endpoint, player: response.gamertag, game: "minecraft" as const },
              ];

        await this.store.set("server_list", serverList);
        ServerListStore.mirrorServerCount(serverList);

        await this.store.delete("auth_state_token");
        await this.store.delete("auth_state_endpoint");
        await this.store.delete(this.PENDING_KEY);
        await this.store.save();

        Analytics.track("LoginCompleted", { game_type: "minecraft" });
        // Replaced, not pushed: the callback carried a single-use code that is now spent.
        window.location.replace("/setup");
    }

    /**
     * Carry on, or say plainly that the sign-in did not finish.
     *
     * Credentials are the only honest answer to "did that work". `server_login` writes them from
     * Rust, so they can exist even when the webview died before recording anything — which is the
     * case this exists for. Their presence for the server this callback was issued against means
     * the exchange completed, whoever was around to see it.
     *
     * `auth_state_endpoint` names that server and is deleted only on success, so it is the more
     * accurate of the two. Falling back to `current_server` covers the exchange that completed
     * and cleared it; without the fallback a duplicate arriving after a clean login would report
     * a failure to somebody who is signed in.
     */
    private async resumeOrFail(): Promise<void> {
        const endpoint =
            (await this.store.get<string>("auth_state_endpoint")) ??
            (await this.store.get<string>("current_server"));

        if (!endpoint) {
            await this.failLogin(I18n.t("That sign-in could not be completed. Please sign in again."));
            return;
        }

        let credentials: LoginResponse;
        try {
            credentials = await invoke<LoginResponse>("get_credentials", { server: endpoint });
        } catch (e) {
            logError(`AuthCallbackHandler: no session for ${endpoint} after a spent code: ${e}`);
            await this.failLogin(I18n.t("That sign-in could not be completed. Please sign in again."));
            return;
        }

        if (!credentials?.certificate) {
            await this.failLogin(I18n.t("That sign-in could not be completed. Please sign in again."));
            return;
        }

        info(`AuthCallbackHandler: session for ${endpoint} already exists, continuing`);
        await this.establishSession(endpoint, credentials);
    }

    private async alreadyRedeemed(code: string): Promise<boolean> {
        const seen = (await this.store.get<string[]>(this.REDEEMED_KEY)) ?? [];
        return seen.includes(this.fingerprint(code));
    }

    /**
     * Record this code as spent.
     *
     * Written before the exchange, not after: a code is spent the moment it is sent, so a
     * failed exchange must not leave it looking available. Retrying it cannot succeed, and
     * the failure it produces is indistinguishable from a rejected account.
     *
     * Stored as a fingerprint. A whole authorization code is a live credential until it is
     * redeemed, and this only ever needs to answer whether two of them are the same one.
     */
    private async claimCode(code: string): Promise<void> {
        const seen = (await this.store.get<string[]>(this.REDEEMED_KEY)) ?? [];
        const kept = [...seen, this.fingerprint(code)].slice(-this.REDEEMED_LIMIT);
        await this.store.set(this.REDEEMED_KEY, kept);
        await this.store.save();
    }

    private fingerprint(code: string): string {
        return code.slice(-32);
    }

    /**
     * The server rejected the account itself (403: not registered on the server,
     * or banished). This is not recoverable by retrying, so it gets a hard error
     * page rather than an inline login-form warning.
     */
    private async denyLogin(): Promise<void> {
        try {
            await this.store.delete(this.PENDING_KEY);
            await this.store.save();
        } catch (e) {
            logError(`AuthCallbackHandler: Failed to clear pending deep link: ${e}`);
        }
        window.location.replace("/error?code=AUTH02");
    }

    /**
     * The sign-in succeeded and the credentials could not be persisted. Not recoverable by
     * retrying until the device's secure storage is fixed, so it gets a hard error page with
     * the remedy rather than an inline login-form warning.
     */
    private async failStorage(code: string): Promise<void> {
        try {
            await this.store.delete(this.PENDING_KEY);
            await this.store.save();
        } catch (e) {
            logError(`AuthCallbackHandler: Failed to clear pending deep link: ${e}`);
        }
        window.location.replace(`/error?code=${code}`);
    }

    /**
     * Persist a user-facing error, clear the pending callback so its single-use
     * code is not retried, and return to the login page. The login page surfaces
     * the persisted error on mount (consumeAuthError) and live (watchAuthError),
     * so the failure is shown whether or not the page was still mounted.
     */
    private async failLogin(message: string): Promise<void> {
        try {
            await this.store.set(this.LOGIN_ERROR_KEY, message);
            await this.store.delete(this.PENDING_KEY);
            await this.store.save();
        } catch (e) {
            logError(`AuthCallbackHandler: Failed to persist login error: ${e}`);
        }
        // When the login page is already mounted, its watchAuthError listener has
        // consumed the persisted key and rendered the message. Reloading here would
        // wipe that render, and the remount's consumeAuthError would find the key
        // already cleared - the failure would never be shown. Only navigate when
        // the callback landed on some other page.
        const path = window.location.pathname.replace(/\/+$/, "");
        if (path !== "/login") {
            window.location.replace("/login");
        }
    }
}
