import { I18n } from "$lib/i18n";

import { Store } from '@tauri-apps/plugin-store';
import { info, error, warn } from '@tauri-apps/plugin-log';
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

import type { AudioDevice } from "../../js/bindings/AudioDevice.ts";
import type { LoginResponse } from "../../js/bindings/LoginResponse.ts";
import BVCApp from "./BVCApp";
import { AppStore } from "./services/AppStore";
import SetupFlow from './setup/SetupFlow';
import PlatformDetector from './utils/PlatformDetector';
import AgeGateService from './services/AgeGateService';
import { PublicServerConfig } from './services/PublicServerConfig';
import FeatureFlagService from './services/FeatureFlagService';
import ImageCache from './components/imageCache';
import ImageCacheOptions from './components/imageCacheOptions';

import { PlayerManager } from './managers/PlayerManager';
import ChannelManager from './managers/ChannelManager';
import { AudioActivityManager } from './managers/AudioActivityManager';
import { SelfController } from './dashboard/SelfController';
import { RailView, type RailServer } from './dashboard/RailView';
import { NearbyManager } from './dashboard/NearbyManager';
import { PlayerLevelSources } from './dashboard/PlayerLevelSources';
import type { ScreenLanding } from './shell/ScreenLanding';
import { BootTimeline } from './shell/BootTimeline';
import { BootProgress } from './shell/BootProgress';
import Analytics from './analytics';
import type { KeybindConfig } from '../bindings/KeybindConfig.ts';
import type { NoiseGateSettings } from '../bindings/NoiseGateSettings.ts';
import { NoiseGateModel } from './settings/NoiseGateModel.ts';
import type { ApiConfigCheckResponse } from '../bindings/ApiConfigCheckResponse.ts';
import type { ServerListEntry } from '../bindings/ServerListEntry.ts';
import { WebSocketSettingsManager } from './managers/settings/WebSocketSettingsManager';
import GameNameUtils from './utils/GameNameUtils';

import {
  checkPermission,
  startForegroundService,
  stopForegroundService,
  updateNotification,
  PermissionType,
  isServiceRunning,
  type ServiceResponse,
  type ServiceStatusResponse,
} from 'tauri-plugin-audio-permissions';

declare global {
  interface Window {
    App: any;
  }
}

export default class Dashboard extends BVCApp {
    private store: Store | undefined;
    private eventUnlisteners: (() => void)[] = [];
    private currentServerCredentials: LoginResponse | null = null;
    private popperProfile: any = null;
    private setup: SetupFlow | undefined;
    private ageGate = new AgeGateService(new FeatureFlagService());

    // Manager instances for dependency injection
    public playerManager: PlayerManager | undefined;
    public channelManager: ChannelManager | undefined;
    public audioActivityManager: AudioActivityManager | undefined;
    public platformDetector: PlatformDetector | undefined;
    public selfController: SelfController | undefined;
    public nearby: NearbyManager | undefined;
    public levels: PlayerLevelSources | undefined;

    /** The rail's servers, resolved during initialize so the shell can draw immediately. */
    public rail: readonly RailServer[] = [];
    public currentServer = '';
    public gamertag = '';
    /** The game this session authenticated against. */
    public activeGame = 'minecraft';

    /**
     * This client's canonical `game:gamertag`.
     *
     * The screen holds the bare gamertag to show it, and needs this to recognise itself in
     * anything the server sent: channel membership, the position feed and the gain store all
     * carry the certificate's form.
     */
    public get identity(): string {
        return GameNameUtils.canonical(this.gamertag, this.activeGame);
    }

    /**
     * The server's own proximity range and how far the position feed reaches past it.
     *
     * Read from `/api/config` rather than assumed, so the boundary the roster draws is the
     * boundary the audio router uses. The kit's 80 m default belongs to a reference page; a
     * real server's is 48 unless its operator says otherwise.
     */
    public voiceRange = 48;
    public feedScope = 120;

    // Per-server age gate. Fetches the server's declared minimum age from
    // /api/config and asks AgeGateService for a decision. Fail-open: any error,
    // an absent minimum, or a disabled global flag returns false (not blocked).
    private async isAgeBlocked(server: string, credentials: LoginResponse | null): Promise<boolean> {
        if (!credentials) {
            return false;
        }
        try {
            await invoke("api_initialize_client", {
                endpoint: server,
                cert: credentials.certificate_ca,
                pem: credentials.certificate + credentials.certificate_key,
            });
            const config = await invoke<ApiConfigCheckResponse>("api_get_config", { server });
            const ageMinimum = config?.config?.age?.minimum ?? null;
            if (ageMinimum == null) {
                return false;
            }
            return (await this.ageGate.evaluate(ageMinimum)) === "block";
        } catch (e) {
            warn(`Age gate check failed, proceeding: ${e}`);
            return false;
        }
    }

    /**
     * Is the server there at all?
     *
     * Asked before anything slow, and answered by the one request a BVC server serves to
     * anybody — no credentials, no pooled client, and outside the endpoint's circuit
     * breaker, so it reports the server rather than the state of an earlier verdict about
     * it.
     *
     * A boot against a server that is down otherwise spends its whole budget finding that
     * out one call at a time: a credential refresh, an age-gate config read and a QUIC
     * handshake, each of which swallows the failure and carries on, and only the last of
     * them redirects anywhere. That runs past the preloader's ten-second escape hatch, so
     * the screen a stopped server produced was "The app may be stuck" rather than the fault
     * page naming it.
     */
    private async answers(server: string): Promise<boolean> {
        try {
            await PublicServerConfig.read(server);
            return true;
        } catch (e) {
            warn(`Server ${server} did not answer: ${e}`);
            return false;
        }
    }

    /**
     * Where a failure inside the boot sequence leads.
     *
     * Recorded rather than performed, because several of these are decided inside callbacks
     * and `.catch` blocks that cannot return a landing to `initialize`. It reports its
     * destination there instead, so the boot overlay stays up over the redirect rather than
     * lifting to show a dashboard that is already on its way somewhere else.
     */
    private pendingHref: string | null = null;

    private redirect(href: string): ScreenLanding {
        this.pendingHref = href;
        return { kind: 'navigate', href };
    }

    async initialize(): Promise<ScreenLanding> {
        const timeline = BootTimeline.shared();
        timeline.mark("dashboard route mounted");
        const progress = BootProgress.shared();
        progress.step("Session", "running");

        this.store = await AppStore.load();
        this.platformDetector = new PlatformDetector();
        timeline.mark("Store.load");

        // Stop any recording that was active before a page refresh.
        // The Tauri backend persists across webview reloads, so a recording
        // could still be running silently from the previous session.
        try {
            const wasRecording = await invoke<boolean>('is_recording');
            if (wasRecording) {
                await invoke('stop_recording');
                info(I18n.t("Stopped recording that was active before page refresh"));
            }
        } catch (e) {
            warn(`Failed to check/stop recording on refresh: ${e}`);
        }
        timeline.mark("is_recording");

        // Device setup is re-checked on every launch: permissions and audio hardware
        // change underneath the app, and the OS is the source of truth for both.
        this.setup = new SetupFlow(this.store);
        await this.setup.initialize();
        timeline.mark("SetupFlow.initialize");

        info("Setup complete: " + this.setup.isComplete());
        if (!this.setup.isComplete()) {
            info("Redirecting to setup");
            return this.redirect("/setup");
        }

        Analytics.track("DashboardReached");

        const appWebview = getCurrentWebviewWindow();

        // `?server=` is the rail's switch, and it names the server this boot is for. Applied
        // before anything reads the store, because the connect applied it afterwards: every
        // step from the certificate check to the dial ran against the server being switched
        // away from, and only the boot after that one arrived anywhere new.
        const requested = new URLSearchParams(window.location.search).get("server");
        if (requested) {
            await this.store.set("current_server", requested);
            await this.store.save();
            info(`Server changed to ${requested}`);
        }

        const currentServer = await this.store.get<string>("current_server");
        timeline.mark("store.get current_server");

        // Check certificate validity before initializing anything that depends on a valid session
        if (currentServer) {
            try {
                const expired = await invoke<boolean>("is_certificate_expired", { server: currentServer });
                if (expired) {
                    warn("Certificate expired for " + currentServer + ", logging out and redirecting to login");
                    await invoke("logout");
                    return this.redirect("/login?reauth=true&server=" + currentServer);
                }
            } catch (e) {
                warn("Could not check certificate expiry: " + e);
            }
        }
        timeline.mark("is_certificate_expired");
        progress.step("Session", "ok");
        progress.step("Server", "running");

        if (currentServer && !(await this.answers(currentServer))) {
            progress.step("Server", "bad", "no response");
            progress.skipFrom("Voice path");
            return this.redirect("/error?code=CONN01");
        }
        progress.step("Server", "ok");
        timeline.mark("reachability (public /api/config)");

        // Initialize managers with dependency injection
        await this.initializeManagers();
        timeline.mark("initializeManagers");

        // The operator-facing server is always on, so there is no enable flag left to consult.
        // What boot still owes it is a token: nothing else mints one now that the enable step is
        // gone, and the listener declines to bind without it. Initializing the manager here is
        // what gives a fresh install a working server before anyone opens its settings pane.
        await new WebSocketSettingsManager().initialize().catch((e) => {
            error(`Error preparing the WebSocket server: ${e}`);
        });
        timeline.mark("websocket server");

        // Every platform, not just desktop.
        //
        // Registering global shortcuts is the desktop-only half; applying the voice mode is
        // not. It is what mutes the input on the way into push-to-talk, and skipping it left
        // a phone that had push-to-talk saved booting with an open microphone — and with the
        // backend believing the mode was open mic, so every hold it was later asked for was
        // refused.
        const keybindConfig = await this.store!.get<KeybindConfig>("keybinds") ?? {
            toggleMute: "ControlLeft+BracketLeft",
            toggleDeafen: "ControlLeft+BracketRight",
            toggleRecording: "ControlLeft+Backslash",
            pushToTalk: "Backquote",
            voiceMode: "openMic",
        };
        await invoke('start_keybind_listener', { config: keybindConfig }).catch((e) => {
            error(`Error starting keybind listener: ${e}`);
        });
        timeline.mark("keybinds (store.get + start_keybind_listener)");

        // If the audio engine is stopped for either the input or output channel, shutdown the existing one, reinitialize everything
        if (currentServer) {
            this.currentServerCredentials = await invoke<LoginResponse>("get_credentials", { server: currentServer });

            // Refresh server permissions and handle certificate re-issuance
            try {
                const activeGame = await this.store.get<string>("active_game");
                await invoke("refresh_server_state", { game: activeGame ?? undefined });
                // Re-fetch credentials since refresh_server_state persists updates to keyring
                this.currentServerCredentials = await invoke<LoginResponse>("get_credentials", { server: currentServer });
            } catch (e) {
                warn(I18n.t("Failed to refresh server state, using cached permissions"));
            }
            timeline.mark("credentials x2 + refresh_server_state (NETWORK)");

            if (await this.isAgeBlocked(currentServer, this.currentServerCredentials)) {
                return this.redirect("/error?code=AGE01");
            }
            timeline.mark("age gate (api_initialize_client + api_get_config)");

            const isInputStreamStopped = await invoke("is_stopped", { device: "InputDevice" }).then((stopped) => stopped as boolean);
            const isOutputStreamStopped = await invoke("is_stopped", { device: "OutputDevice" }).then((stopped) => stopped as boolean);
            timeline.mark("is_stopped x2");

            if (isInputStreamStopped || isOutputStreamStopped) {
                progress.step("Permissions", "running");
                await this.shutdown();
                timeline.mark("shutdown (audio teardown)");

                // Check audio permission first
                info(I18n.t("Checking audio permission..."));
                const audioPermission = await checkPermission({ permissionType: PermissionType.Audio });

                if (!audioPermission.granted) {
                    warn(I18n.t("Audio permission denied"));
                    progress.step("Permissions", "bad", "microphone denied");
                    return this.redirect("/error?code=PERM1");
                }

                const notificationGranted = await checkPermission({ permissionType: PermissionType.Notification });

                if (!notificationGranted.granted) {
                    warn(I18n.t("Notification permission denied - notifications may not be visible"));
                    progress.step("Permissions", "bad", "notifications denied");
                    return this.redirect("/error?code=PERM2");
                }

                // On mobile we need a running background service to allow microphone capture if the
                // application is backgrounded.
                const isServiceRunningResult: ServiceStatusResponse = await isServiceRunning();
                if (!isServiceRunningResult.running) {
                    const serviceResult: ServiceResponse = await startForegroundService({
                        onPermissionRevoked: (event) => {
                            warn(`Permission revoked: ${event.permissionType}`);
                            this.redirect("/error?code=PERM1");
                        }
                    });

                    if (!serviceResult.started) {
                        warn(I18n.t("Foreground service could not be started."));
                        progress.step("Permissions", "bad", "background service");
                        return this.redirect("/error?code=SERV01");
                    }
                }

                progress.step("Permissions", "ok");
                timeline.mark("permissions + foreground service (OS)");

                // Initialize audio devices and network stream
                await this.initializeAudioDevicesAndNetworkStream(this.store!, currentServer ?? "", this.currentServerCredentials);

                // Re-initialize AudioActivityManager since shutdown() destroyed it
                if (this.audioActivityManager) {
                    await this.audioActivityManager.initialize();
                }

                // shutdown() also unregistered PlayerManager's gain-store
                // listener; without re-registering, every COLD start ships a
                // dashboard whose player cards never react to in-game
                // volume/hear changes (warm re-entries skip this branch, which
                // is why navigating away and back "fixed" it).
                if (this.playerManager) {
                    await this.playerManager.listenForBackendUpdates();
                }

                // shutdown() also stopped the level fan-out and the self
                // controller. The meters hold their sources from mount, so the
                // objects are reused — but a stopped PlayerLevelSources is
                // unsubscribed from the feed, and every COLD start shipped a
                // pill and roster whose meters never moved again while the
                // level events kept arriving in the window.
                if (this.levels) {
                    await this.levels.start();
                }
                if (this.selfController) {
                    await this.selfController.start();
                }

                // Update notification
                await updateNotification({
                    title: "Bedrock Voice Chat",
                    message: I18n.t("In public voice chat")
                });
            } else {
                // Nothing was asked of the operating system: both streams were already
                // running, which is what a warm re-entry looks like.
                progress.step("Permissions", "skipped", "already granted");
            }
        }

        // A failure decided inside a callback or a `.catch` reports its destination here,
        // because it could not return one from where it happened.
        if (this.pendingHref) {
            return { kind: 'navigate', href: this.pendingHref };
        }

        timeline.mark("post-connect tail (activity mgr, listeners, notification)");

        await this.loadIdentity();
        timeline.mark("loadIdentity");

        await this.startNearby();
        timeline.mark("startNearby (position feed)");

        return { kind: 'show' };
    }

    /**
     * The rail's servers and who you are, read once the session is settled.
     *
     * Separate from `initializeManagers` because it runs after `refresh_server_state` may
     * have reissued credentials: reading the gamertag before that would show the name from
     * a certificate that is no longer the one in use.
     */
    private async loadIdentity(): Promise<void> {
        if (!this.store) return;
        const saved = (await this.store.get<ServerListEntry[]>("server_list")) ?? [];
        this.currentServer = (await this.store.get<string>("current_server")) ?? '';
        this.rail = RailView.rows(saved, this.currentServer);
        this.gamertag =
            this.currentServerCredentials?.gamertag ??
            (await this.store.get<string>("current_player")) ??
            '';
    }

    /**
     * Open the position feed and the per-player level adapter.
     *
     * Last, because it needs both the server's range and a settled session: a feed opened
     * before `refresh_server_state` could reissue credentials would be holding a ticket
     * bought with a certificate that is no longer in use.
     */
    /**
     * The one object that distributes levels, created before anything asks for one.
     *
     * `initializeManagers` runs long before `startNearby`, and the controller needs a source
     * for the pill at construction — which is exactly why the pill grew a parallel mechanism
     * of its own. Ensuring it here means both callers get the same instance whichever runs
     * first, and a reconnect reuses it rather than leaving every mounted meter bound to one
     * nothing writes to.
     */
    private levelSources(): PlayerLevelSources {
        if (!this.levels) {
            this.levels = new PlayerLevelSources();
            void this.levels.start();
        }
        return this.levels;
    }

    private async startNearby(): Promise<void> {
        if (!this.store || !this.currentServer) return;

        // Both are reused rather than replaced, because a reconnect calls this again and the
        // screen is already holding references into them: a level source is handed to a meter
        // at mount, and the roster subscribes to these stores once at boot. Swapping either
        // object leaves every card on screen bound to one that nothing writes to any more.
        this.levelSources();
        if (!this.nearby) {
            this.nearby = new NearbyManager();
        }

        // `start` stops itself first, so re-entering it re-opens the feed on a fresh ticket
        // without stranding the old socket.
        await this.nearby.start(this.currentServer, this.voiceRange);
    }

    showPreloader(): void {
        this.preloader();
    }

    /**
     * Retry the voice connection in place.
     *
     * Runs the whole connect sequence, not just the QUIC dial. `changeNetworkStream` alone
     * re-dials and leaves everything downstream of the old session in place: the audio streams
     * keep their per-speaker jitter buffers and decoder state from a connection that no longer
     * exists, the position feed goes on retrying against a ticket bought with the previous
     * session, and the channel list is whatever it was before the server went away. Pressing
     * Reconnect appeared to work and nobody could be heard afterwards.
     *
     * `initializeAudioDevicesAndNetworkStream` is the same function boot uses, deliberately:
     * one path for connecting means a reconnect cannot drift out of step with a cold start.
     *
     * Its failures are recorded as a destination rather than thrown, which is right for boot and
     * wrong for a button inside a status readout — a retry that fails should say so where it was
     * pressed rather than throwing the whole screen away. So the recorded destination is
     * restored afterwards and reported as a return value instead.
     */
    async reconnect(): Promise<boolean> {
        if (!this.store || !this.currentServer) return false;
        const before = this.pendingHref;
        try {
            // Closed before the dial rather than after: the feed's retry would otherwise spend
            // the reconnect asking for tickets the dead session cannot authorise.
            this.nearby?.stop();

            await this.initializeAudioDevicesAndNetworkStream(
                this.store,
                this.currentServer,
                this.currentServerCredentials,
            );

            // The world moved on while the link was down. Membership is the server's to state.
            await this.channelManager?.initialize();
            await this.startNearby();
        } catch (e) {
            warn(`Reconnect failed: ${e}`);
        }
        const failed = this.pendingHref !== before;
        this.pendingHref = before;
        return !failed;
    }

    /** The current server's hostname, which is what the glyph is derived from. */
    host(): string {
        return this.currentServer.replace(/^https?:\/\//, '').replace(/\/$/, '');
    }

    /**
     * Sign out of this server and go to the sign-in.
     *
     * Returns the destination rather than assigning it: the same reason `initialize` does,
     * and it keeps the audio shutdown ordered ahead of the navigation instead of racing it.
     */
    async signOut(): Promise<ScreenLanding> {
        try {
            await invoke("logout");
            await this.shutdown();
        } catch (e) {
            error(`Logout failed: ${e}`);
        }
        return { kind: 'navigate', href: '/login' };
    }

    /**
     * Initialize all managers with proper dependency injection
     */
    private async initializeManagers(): Promise<void> {
        const timeline = BootTimeline.shared();
        if (!this.store) {
            throw new Error('Store must be initialized before managers');
        }

        try {
            // Load static configuration from store for DI
            const currentPlayer = await this.store.get("current_player") as string | null;
            const currentServer = await this.store.get("current_server") as string | null;
            const currentUser = currentPlayer || '';
            const serverUrl = currentServer || '';
            // Every key PlayerManager writes is prefixed with this, so it has to be the game the
            // session actually authenticated against — the login paths all persist it.
            this.activeGame = (await this.store.get("active_game") as string | null) || 'minecraft';

            timeline.mark('  ↳ managers: store.get x3');
            this.playerManager = new PlayerManager(currentUser, this.activeGame);
            await this.playerManager.listenForBackendUpdates();
            timeline.mark('  ↳ managers: player backend listener');
            this.channelManager = new ChannelManager(this.playerManager, this.store, serverUrl);

            // Reused rather than replaced, for the reason `startNearby` gives about the level
            // sources: the pill is handed `micSource` at mount and the status panel subscribes
            // to `diagnostics` once, so a reconnect that swapped this object left both bound to
            // an instance nothing writes to any more — a meter that never moves and a readout
            // that reports no events, over a microphone that is working.
            //
            // `start` is idempotent enough to re-enter: it re-seeds from the backend and
            // re-attaches the level listener, which is what a reconnect needs anyway.
            if (!this.selfController) {
                this.selfController = new SelfController(this.store, this.levelSources());
            }
            await this.selfController.start();
            timeline.mark('  ↳ managers: self controller');

            // Initialize AudioActivityManager (independent)
            this.audioActivityManager = new AudioActivityManager(this.store);
            await this.audioActivityManager.initialize();
            timeline.mark('  ↳ managers: audio activity');
        } catch (err) {
            error(`Dashboard: Failed to initialize managers: ${err}`);
            throw err;
        }
    }

    /**
     * Get managers for dependency injection into components
     */
    getManagers() {
        return {
            playerManager: this.playerManager,
            channelManager: this.channelManager,
            audioActivityManager: this.audioActivityManager
        };
    }

    /**
     * Set the player avatar after DOM is ready
     */
    public async setPlayerAvatar(): Promise<void> {
        if (!this.currentServerCredentials) {
            warn('No current server credentials available for avatar');
            return;
        }

        // Set player avatar with proper base64 validation
        const avatarElement = document.getElementById("player-sidebar-avatar");
        const dropdownAvatarElement = document.getElementById("player-dropdown-avatar");
        const dropdownNameElement = document.getElementById("player-dropdown-name");
        const profileButton = document.getElementById("profile-ref");

        if (avatarElement && this.store) {
            let avatarSrc = "";

            if (this.currentServerCredentials?.gamerpic) {
                try {
                    // Normalize: existing keyring data may be base64-encoded
                    let avatarUrl = this.currentServerCredentials.gamerpic;
                    if (!avatarUrl.startsWith('http')) {
                        try { avatarUrl = atob(avatarUrl); } catch { }
                    }

                    const imageCache = new ImageCache();
                    const options = new ImageCacheOptions(avatarUrl, 86400);
                    avatarSrc = await imageCache.getImage(options);
                } catch (err) {
                    warn(`Dashboard: Failed to fetch/decode player avatar: ${err}`);
                }
            }

            avatarElement.setAttribute("src", avatarSrc);
            if (dropdownAvatarElement) {
                dropdownAvatarElement.setAttribute("src", avatarSrc);
            }

            if (avatarSrc && this.channelManager) {
                this.channelManager.setCurrentUserGamepic(avatarSrc);
            }
        }

        // Set player name in dropdown
        if (dropdownNameElement && this.currentServerCredentials?.gamertag) {
            dropdownNameElement.textContent = this.currentServerCredentials.gamertag;
        }

        // Set game name in dropdown (first letter capitalized)
        const dropdownGameElement = document.getElementById("player-dropdown-game");
        if (dropdownGameElement && this.store) {
            const activeGame = await this.store.get<string>("active_game");
            if (activeGame) {
                // Capitalize first letter
                const gameName = activeGame.charAt(0).toUpperCase() + activeGame.slice(1);
                dropdownGameElement.textContent = gameName;
            }
        }

        if (profileButton) {
            const config = {
                placement: "right-end",
                modifiers: [
                    {
                        name: "offset",
                        options: {
                            offset: [0, 12],
                        },
                    },
                ],
            };

            if (typeof (window as any).Popper !== 'undefined') {
                this.popperProfile = new (window as any).Popper(
                    '#profile-wrapper',
                    '#profile-ref',
                    '#profile-box',
                    config
                );
            }

            const logoutButton = document.getElementById('logout-button');
            if (logoutButton) {
                logoutButton.addEventListener("click", this.handleLogout.bind(this));

                this.eventUnlisteners.push(() => {
                    logoutButton.removeEventListener("click", this.handleLogout.bind(this));
                    if (this.popperProfile && this.popperProfile.destroy) {
                        this.popperProfile.destroy();
                    }
                });
            }
        }
    }

    /**
     * Handle logout action
     */
    private async handleLogout(): Promise<void> {
        try {
            Analytics.track("Logout");
            await invoke("logout").then(async () => {
                await this.shutdown().then(() => {
                    // Replaced, not pushed: the session is gone, so the dashboard behind
                    // this cannot be returned to.
                    window.location.replace("/login");
                });
            });

        } catch (err) {
            const appWebview = getCurrentWebviewWindow();
            await appWebview.emit('notification', {
                title: I18n.t("Logout Failed"),
                body: I18n.t("An error occurred during logout. Please try again."),
                level: "error"
            });
        }
    }

    async initializeAudioDevicesAndNetworkStream(store: Store, currentServer: string, credentials: LoginResponse | null): Promise<void> {
        if (currentServer) {
            // Update the current player information, then we can render the dashboard views with it
            await invoke("update_stream_metadata", {
                key: "current_player",
                value: credentials?.gamertag ?? "",
                device: "OutputDevice"
            }).then(async () => {

                // Update PlayerManager with current user
                if (this.playerManager && credentials?.gamertag) {
                    this.playerManager.setCurrentUser(credentials.gamertag);
                }

                // Load any metadata from the settings store
                let useNoiseGate = await store.get("use_noise_gate") as boolean | null;
                if (useNoiseGate == null) {
                    await store.set("use_noise_gate", false);
                    await store.save();
                    useNoiseGate = false;
                }

                // Seeded from the same constant the settings screen resets to. These were
                // two separate literals, and the launch gate was therefore not the gate
                // Reset restored — a microphone that passed nothing until it was reset.
                let noiseGateSettings = NoiseGateModel.hydrate(
                    await store.get("noise_gate_settings") as Partial<NoiseGateSettings> | null,
                );
                await store.set("noise_gate_settings", noiseGateSettings);
                await store.save();

                // Set the noise gate
                await invoke("update_stream_metadata", {
                    key: "use_noise_gate",
                    value: useNoiseGate ? "true" : "false",
                    device: "InputDevice",
                });

                await invoke("update_stream_metadata", {
                    key: "noise_gate_settings",
                    value: JSON.stringify(noiseGateSettings),
                    device: "InputDevice"
                });

                // Seed the mixer with this server's persisted volumes. The projection starts
                // empty, so until this runs every mute the user set is inert.
                await invoke("player_settings_publish");

                // Fetch server config to get fresh QUIC port and spatial audio settings
                try {
                    const configResponse = await invoke<ApiConfigCheckResponse>("api_get_config", { server: currentServer });

                    // Update QUIC port from server config
                    if (configResponse?.config?.quic_port && credentials) {
                        const freshPort = configResponse.config.quic_port.toString();
                        if (credentials.quic_connect_string !== freshPort) {
                            info(`Updating QUIC port from ${credentials.quic_connect_string} to ${freshPort}`);
                            credentials.quic_connect_string = freshPort;

                            await invoke("set_credential", {
                                server: currentServer,
                                key: "quic_connect_string",
                                value: freshPort
                            });
                        }
                    }

                    if (configResponse?.config?.spatial_audio) {
                        await invoke("update_stream_metadata", {
                            key: "spatial_audio_config",
                            value: JSON.stringify(configResponse.config.spatial_audio),
                            device: "OutputDevice"
                        });

                        // The metadata above does not survive the app closing, and a recording is
                        // usually exported after the session it came from has ended. Without this
                        // copy that export renders on the compiled falloff curve rather than this
                        // server's, and nothing in the output would say so.
                        await store.set("spatial_audio_config", configResponse.config.spatial_audio);
                        await store.save();

                        // The same number the audio router uses, so the line the roster draws
                        // and the line a voice actually stops at are one line. The feed reaches
                        // 2.5x past it, which is what gives the ring an approach to animate.
                        this.voiceRange = configResponse.config.spatial_audio.broadcast_range;
                        this.feedScope = Math.min(256, this.voiceRange * 2.5);
                    }
                } catch (e) {
                    warn(`Failed to fetch server config, using stored values: ${e}`);
                }

                BootTimeline.shared().mark("stream metadata + api_get_config (pre-connect)");

                BootProgress.shared().step("Voice path", "running");
                await this.changeNetworkStream(currentServer, credentials);
                BootTimeline.shared().mark(">>> QUIC HANDSHAKE (change_network_stream) <<<");
                BootProgress.shared().step("Audio", "running");

                await this.updateAudioDevice("OutputDevice");
                await this.updateAudioDevice("InputDevice");
                BootTimeline.shared().mark("updateAudioDevice x2");
                await invoke("change_audio_device").catch((e) => {
                    const errStr = String(e);
                    if (errStr.includes("INCOMPATIBLE_DEVICE")) {
                        error(`Incompatible audio device: ${e}`);
                        BootProgress.shared().step("Audio", "bad", "incompatible device");
                        this.redirect("/error?code=AUDI01");
                        return;
                    }
                    if (errStr.includes("NO_INPUT_DEVICE")) {
                        error(`No input device available: ${e}`);
                        BootProgress.shared().step("Audio", "bad", "no input device");
                        this.redirect("/error?code=AUDI02");
                        return;
                    }
                    if (errStr.includes("NO_OUTPUT_DEVICE")) {
                        error(`No output device available: ${e}`);
                        BootProgress.shared().step("Audio", "bad", "no output device");
                        this.redirect("/error?code=AUDI03");
                        return;
                    }
                    error(`Audio device error: ${e}`);
                });
                BootProgress.shared().step("Audio", "ok");
                BootTimeline.shared().mark("change_audio_device (stream start)");
            }).catch((e) => {
                error(`Error updating current player: ${e}`);
            });
        } else {
            warn(I18n.t("No current server found in store!"));
            await this.shutdown();
            this.redirect("/");
        }
    }

    async updateAudioDevice(type: string): Promise<void> {
        await invoke("get_audio_device", { io: type })
            .then(async (device) => device as AudioDevice)
            .then(async (device) => {
                info(`Using ${device.name} as ${type}`);

                await invoke("set_audio_device", { device: device })
                    .then(async () => {
                        info(`Audio device changed to ${device.name} for ${type}`);
                    })
                    .catch((e) => {
                        error(`Error changing audio device: ${e}`);
                        return null;
                    });
            })
            .catch((err) => {
                error(`Error getting audio device: ${err}`);
                return null;
            });
    }

    async changeNetworkStream(currentServer: string, credentials: LoginResponse | null): Promise<void> {
        await invoke("stop_network_stream");
        try {
            await invoke("change_network_stream", { server: currentServer, data: credentials });
            info(`Changed network stream to ${currentServer}`);
            BootProgress.shared().step("Voice path", "ok");
        } catch (e) {
            const errStr = String(e);
            if (errStr.includes("DNS_FAIL")) {
                error(`DNS resolution failed: ${e}`);
                BootProgress.shared().step("Voice path", "bad", "DNS lookup failed");
                this.redirect("/error?code=DNS01");
            } else if (errStr.includes("CERT_INVALID")) {
                // Both certificate branches are checked ahead of QUIC_FAIL: the firewall advice
                // QUIC01 gives would send the user to fix something that is not broken.
                error(`Server certificate rejected, credentials cleared: ${e}`);
                BootProgress.shared().step("Voice path", "bad", "certificate rejected");
                this.redirect("/error?code=CERT01");
            } else if (errStr.includes("SERVER_CERT")) {
                error(`Server voice certificate is misconfigured, credentials kept: ${e}`);
                BootProgress.shared().step("Voice path", "bad", "server certificate");
                this.redirect("/error?code=CERT02");
            } else if (errStr.includes("QUIC_FAIL")) {
                error(`QUIC connection failed: ${e}`);
                BootProgress.shared().step("Voice path", "bad", "no voice transport");
                this.redirect("/error?code=QUIC01");
            } else {
                error(`Error changing network stream: ${e}`);
                BootProgress.shared().step("Voice path", "bad", "connect failed");
                this.redirect("/error?code=CONN01");
            }
        }
    }

    async cleanup(): Promise<void> {
        // Clean up managers
        try {
            if (this.nearby) {
                this.nearby.stop();
            }
            if (this.levels) {
                this.levels.stop();
            }
            if (this.selfController) {
                this.selfController.cleanup();
            }
            if (this.channelManager) {
                this.channelManager.cleanup();
            }
            if (this.audioActivityManager) {
                this.audioActivityManager.destroy();
            }
            if (this.playerManager) {
                this.playerManager.cleanup();
            }
        } catch (err) {
            error(`Error cleaning up managers: ${err}`);
        }

        // Clean up other event listeners
        this.eventUnlisteners.forEach(unlisten => {
            try {
                unlisten();
            } catch (err) {
                error(`Error cleaning up event listener: ${err}`);
            }
        });
        this.eventUnlisteners = [];

        // Last, and it was missing: the base holds this instance's deep-link, connection-health
        // and audio-recovery listeners, and they are process-wide — nothing else releases them.
        // Skipping it left a second audio-recovery handler live after every teardown, so one
        // device error invoked `restart_audio_stream` twice.
        await super.cleanup();
    }
}