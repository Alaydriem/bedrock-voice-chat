
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

        document.querySelector("#sidebar-toggle")?.addEventListener("click", (e) => {
            const el = e.target as HTMLElement;
            el.classList.toggle("active");
            document.querySelector("body")?.classList.toggle("is-sidebar-open");
        });
        
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
                debug("Audio engine is stopped, reinitializing...");
                await this.shutdown();
                await requestAudioPermissions().then(async (granted) => {
                    if (granted) {
                        info("Audio permissions are granted by ABI");
                        await this.initializeAudioDevicesAndNetworkStream(this.store!, currentServer ?? "", this.currentServerCredentials);
                    } else {
                        warn("Audio permissions are not granted, or need to be requested");
                    }
                });
                
            }
        }
        // Render the dashboard
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
            
            if (currentUser) {
                info(`Dashboard: Loaded current user from store: ${currentUser}`);
            } else {
                warn('Dashboard: No current user found in store');
            }
            
            if (serverUrl) {
                info(`Dashboard: Loaded current server from store: ${serverUrl}`);
            } else {
                warn('Dashboard: No current server found in store');
            }

            // Initialize PlayerManager first (no dependencies)
            this.playerManager = new PlayerManager(this.store, currentUser);
            info('Dashboard: PlayerManager initialized');

            // Initialize ChannelManager (depends on PlayerManager)
            this.channelManager = new ChannelManager(this.playerManager, this.store, serverUrl);
            info('Dashboard: ChannelManager initialized');

            // Initialize AudioActivityManager (independent)
            this.audioActivityManager = new AudioActivityManager(this.store);
            await this.audioActivityManager.initialize();
            info('Dashboard: AudioActivityManager initialized');
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
        
        if (avatarElement && this.currentServerCredentials?.gamerpic) {
            try {
                info(`Dashboard: Setting player avatar for ${this.currentServerCredentials.gamertag}`);
                const decodedAvatar = atob(this.currentServerCredentials.gamerpic);
                avatarElement.setAttribute("src", decodedAvatar);
                info(`Dashboard: Set player avatar for ${this.currentServerCredentials.gamertag}`);
            } catch (err) {
                warn(`Dashboard: Failed to decode player avatar: ${err}`);
                // Set a default avatar or leave empty
                avatarElement.setAttribute("src", "");
            }
        } else {
            if (!avatarElement) {
                warn(`Dashboard: Avatar element 'player-sidebar-avatar' not found in DOM`);
            }
            if (!this.currentServerCredentials?.gamerpic) {
                warn(`Dashboard: No gamerpic data available`);
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
                info("Updated current player");

                // Update PlayerManager with current user
                if (this.playerManager && credentials?.gamertag) {
                    this.playerManager.setCurrentUser(credentials.gamertag);
                    info(`Dashboard: Set current user in PlayerManager: ${credentials.gamertag}`);
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
                info('Dashboard: ChannelManager cleaned up');
            }
            if (this.audioActivityManager) {
                this.audioActivityManager.destroy();
                info('Dashboard: AudioActivityManager cleaned up');
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

    async bindSidebarEvents() {
        document.querySelector("#reload-audio-engine")?.addEventListener("click", async () => {
            window.App.shutdown();
            window.location.reload();
        });

        const mute_input = document.querySelector("#mute-audio-input");
        const mute_input_fa = document.querySelector("#mute-audio-input i");
        await invoke("mute_status", { device: "InputDevice" }).then((muted) => {
            if (muted) {
                mute_input_fa?.classList.add("fa-microphone-slash");
                mute_input_fa?.classList.remove("fa-microphone");
                mute_input_fa?.classList.add("text-error");
            }
        });
        mute_input?.addEventListener("click", async (el) => {
            invoke("mute", { device: "InputDevice" }).then(() => {
                const i = document.querySelector("#mute-audio-input i");
                i?.classList.toggle("fa-microphone-slash");
                i?.classList.toggle("fa-microphone");
                i?.classList.toggle("text-error");
            });
        });

        const mute_output = document.querySelector("#mute-audio-output");
        const mute_output_fa = document.querySelector("#mute-audio-output i");
        await invoke("mute_status", { device: "OutputDevice" }).then((muted) => {
            if (muted) {
                mute_output_fa?.classList.add("fa-volume-xmark");
                mute_output_fa?.classList.remove("fa-volume-high");
                mute_output_fa?.classList.add("text-error");
            }
        });
        mute_output?.addEventListener("click", async (el) => {
            invoke("mute", { device: "OutputDevice" }).then(() => {
                const i = document.querySelector("#mute-audio-output i");
                i?.classList.toggle("fa-volume-xmark");
                i?.classList.toggle("fa-volume-high");
                i?.classList.toggle("text-error");
            });
        });
    }
}