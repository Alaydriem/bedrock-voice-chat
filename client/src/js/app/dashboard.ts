
import { Store } from '@tauri-apps/plugin-store';
import { info, error, warn, debug } from '@tauri-apps/plugin-log';
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';

import { mount } from "svelte";

import type { AudioDevice } from "../../js/bindings/AudioDevice.ts";
import type { LoginResponse } from "../../js/bindings/LoginResponse.ts";
import App from './app.js';
import Server from './server.ts';
import Keyring from './keyring.ts';
import Sidebar from "./components/dashboard/sidebar.ts";

import { PlayerManager } from './managers/PlayerManager';
import ChannelManager from './managers/ChannelManager';
import { AudioActivityManager } from './managers/AudioActivityManager';

import Notification from "../../components/events/Notification.svelte";
import type { NoiseGateSettings } from '../bindings/NoiseGateSettings.ts';
import type { PlayerGainStore } from '../bindings/PlayerGainStore.ts';

import {
    requestPermission as requestAudioPermissions
} from 'tauri-plugin-audio-permissions';

declare global {
  interface Window {
    App: any;
  }
}

export default class Dashboard extends App {
    private keyring: Keyring | undefined;
    private store: Store | undefined;
    private eventUnlisteners: (() => void)[] = [];
    private currentServerCredentials: LoginResponse | null = null;
    private popperProfile: any = null;

    // Manager instances for dependency injection
    public playerManager: PlayerManager | undefined;
    public channelManager: ChannelManager | undefined;
    public audioActivityManager: AudioActivityManager | undefined;

    async initialize() {
        const appWebview = getCurrentWebviewWindow();

        // Handle notifications
        const notificationUnlisten = await appWebview.listen('notification', (event: { payload?: { title?: string, body?: string, level?: string } }) => {
            info(`Notification received: ${JSON.stringify(event.payload)}`);
            mount(Notification, {
                target: document.querySelector("#notification-container")!,
                props: {
                    title: event.payload?.title || "",
                    body: event.payload?.body || "",
                    level: event.payload?.level || "info"
                }
            });
        });
        this.eventUnlisteners.push(notificationUnlisten);


        this.store = await Store.load("store.json", { autoSave: false });
        const currentServer = await this.store.get<string>("current_server");

        // Initialize managers with dependency injection
        await this.initializeManagers();

        // If the audio engine is stopped for either the input or output channel, shutdown the existing one, reinitialize everything

        if (currentServer) {
            this.keyring = await Keyring.new("servers");

            const server = new Server();
            server.setKeyring(this.keyring, currentServer);
            this.currentServerCredentials = await server.getCredentials();

            const isInputStreamStopped = await invoke("is_stopped", { device: "InputDevice" }).then((stopped) => stopped as boolean);
            const isOutputStreamStopped = await invoke("is_stopped", { device: "OutputDevice" }).then((stopped) => stopped as boolean);
            if (isInputStreamStopped || isOutputStreamStopped) {
                await this.shutdown();
                await requestAudioPermissions().then(async (granted) => {
                    if (granted) {
                        await this.initializeAudioDevicesAndNetworkStream(this.store!, currentServer ?? "", this.currentServerCredentials);
                    } else {
                        warn("Audio permissions are not granted, or need to be requested");
                    }
                });
            }
        }

        this.preloader();
    }

    /**
     * Initialize all managers with proper dependency injection
     */
    private async initializeManagers(): Promise<void> {
        if (!this.store) {
            throw new Error('Store must be initialized before managers');
        }

        try {
            // Load static configuration from store for DI
            const currentPlayer = await this.store.get("current_player") as string | null;
            const currentServer = await this.store.get("current_server") as string | null;
            const currentUser = currentPlayer || '';
            const serverUrl = currentServer || '';

            this.playerManager = new PlayerManager(this.store, currentUser);
            this.channelManager = new ChannelManager(this.playerManager, this.store, serverUrl);

            // Initialize AudioActivityManager (independent)
            this.audioActivityManager = new AudioActivityManager(this.store);
            await this.audioActivityManager.initialize();
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
    public setPlayerAvatar(): void {
        if (!this.currentServerCredentials) {
            warn('Dashboard: No current server credentials available for avatar');
            return;
        }

        // Set player avatar with proper base64 validation
        const avatarElement = document.getElementById("player-sidebar-avatar");
        const dropdownAvatarElement = document.getElementById("player-dropdown-avatar");
        const dropdownNameElement = document.getElementById("player-dropdown-name");
        const profileButton = document.getElementById("profile-ref");

        if (avatarElement && this.currentServerCredentials?.gamerpic) {
            try {
                const decodedAvatar = atob(this.currentServerCredentials.gamerpic);
                avatarElement.setAttribute("src", decodedAvatar);

                // Also set the dropdown avatar
                if (dropdownAvatarElement) {
                    dropdownAvatarElement.setAttribute("src", decodedAvatar);
                }
            } catch (err) {
                warn(`Dashboard: Failed to decode player avatar: ${err}`);
                // Set a default avatar or leave empty
                avatarElement.setAttribute("src", "");
                if (dropdownAvatarElement) {
                    dropdownAvatarElement.setAttribute("src", "");
                }
            }
        }

        // Set player name in dropdown
        if (dropdownNameElement && this.currentServerCredentials?.gamertag) {
            dropdownNameElement.textContent = this.currentServerCredentials.gamertag;
        }

        // Set up dropdown menu functionality
        if (profileButton) {
            // Use LineOne's simple Popper approach
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

            // Initialize with LineOne's Popper pattern
            if (typeof (window as any).Popper !== 'undefined') {
                this.popperProfile = new (window as any).Popper(
                    '#profile-wrapper',
                    '#profile-ref',
                    '#profile-box',
                    config
                );
            }

            // Just handle the logout button click
            const logoutButton = document.getElementById('logout-button');
            if (logoutButton) {
                logoutButton.addEventListener("click", this.handleLogout.bind(this));

                // Store cleanup function
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
            await invoke("logout").then(async () => {
                await this.cleanup().then(() => {
                    window.location.href = "/login";
                });
            });

        } catch (err) {
            error(`Dashboard: Logout failed: ${err}`);
            // Show error notification
            const notificationContainer = document.querySelector("#notification-container");
            if (notificationContainer) {
                mount(Notification, {
                    target: notificationContainer,
                    props: {
                        title: "Logout Failed",
                        body: "An error occurred during logout. Please try again.",
                        level: "error"
                    }
                });
            }
        }
    }

    async renderSidebar(store: Store, currentServer: string): Promise<void> {
        const serverList = await store.get("server_list") as Array<{ server: string, player: string }>;

        if (serverList) {
            const sidebar = new Sidebar(serverList, currentServer);
            await sidebar.render();
        }
    }

    async initializeAudioDevicesAndNetworkStream(store: Store, currentServer: string, credentials: LoginResponse | null): Promise<void> {
        const urlParams = new URLSearchParams(window.location.search);
        if (urlParams.has("server")) {
            await store.set("current_server", urlParams.get("server"));
            await store.save();
            info("Server changed to " + urlParams.get("server"));
        }

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

                let noiseGateSettings = await store.get("noise_gate_settings") as NoiseGateSettings | null;

                if (noiseGateSettings == null) {
                    await store.set("noise_gate_settings", {
                        open_threshold: -36.0,
                        close_threshold: -56.0,
                        release_rate: 150.0,
                        attack_rate: 5.0,
                        hold_time: 150.0
                    });
                    await store.save();
                    noiseGateSettings = await store.get("noise_gate_settings") as NoiseGateSettings | null;
                }

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

                // Update the player gain metadata
                let playerGainStore = await store.get("player_gain_store") as PlayerGainStore | null;
                if (!playerGainStore || typeof playerGainStore !== "object" || Array.isArray(playerGainStore)) {
                    playerGainStore = {};
                    await store.set("player_gain_store", playerGainStore);
                    await store.save();
                }

                await invoke("update_stream_metadata", {
                    key: "player_gain_store",
                    value: JSON.stringify(playerGainStore),
                    device: "OutputDevice"
                });

                await this.changeNetworkStream(currentServer, credentials);

                await this.updateAudioDevice("OutputDevice");
                await this.updateAudioDevice("InputDevice");
                await invoke("change_audio_device");
            }).catch((e) => {
                error(`Error updating current player: ${e}`);
            });
        } else {
            warn("No current server found in store!");
            await this.shutdown();
            window.location.href = "/server";
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
            .catch((error) => {
                error(`Error getting audio device: ${error}`);
                return null;
            });
    }

    async changeNetworkStream(currentServer: string, credentials: LoginResponse | null): Promise<void> {
        await invoke("stop_network_stream").then(async () => {
            await invoke("change_network_stream", { server: currentServer, data: credentials })
                .then(() => {
                    info(`Changed network stream to ${currentServer}`);
                }).catch((e) => {
                    error(`Error changing network stream: ${e}`);
                });
        });
    }

    async cleanup(): Promise<void> {
        // Clean up managers
        try {
            if (this.channelManager) {
                this.channelManager.cleanup();
            }
            if (this.audioActivityManager) {
                this.audioActivityManager.destroy();
            }
            // PlayerManager doesn't need explicit cleanup currently
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
    }

    async shutdown() {
        await this.cleanup();
        await invoke("reset_asm");
        await invoke("reset_nsm");
    }
}