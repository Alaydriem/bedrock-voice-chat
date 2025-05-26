import Hold from "../../js/app/stronghold.ts";
import { Store } from '@tauri-apps/plugin-store';
import { info, error, warn } from '@tauri-apps/plugin-log';
import { invoke } from "@tauri-apps/api/core";
import type { AudioDevice } from "../../js/bindings/AudioDevice.ts";
import type { AudioDeviceType } from "../../js/bindings/AudioDeviceType.ts";
import type { LoginResponse } from "../../js/bindings/LoginResponse.ts";

import App from './app.js';
import Sidebar from "./components/dashboard/sidebar.ts";

declare global {
  interface Window {
    App: any;
  }
}

export default class Dashboard extends App {
    private stronghold: Hold | undefined;
    private store: Store | undefined;
    
    async initialize() {
        document.querySelector("#sidebar-toggle")?.addEventListener("click", (e) => {
            const el = e.target as HTMLElement;
            el.classList.toggle("active");
            document.querySelector("body")?.classList.toggle("is-sidebar-open");
        });
        this.store = await Store.load("store.json", { autoSave: false });
        const currentServer = await this.store.get<string>("current_server");

        let currentServerCredentials: LoginResponse | null = null;
        if (currentServer) {
            await this.renderSidebar(this.store, currentServer ?? "");
            
            const passwordStore = await Store.load('secrets.json', { autoSave: false });
            const password = await passwordStore.get<string>("stronghold_password");
            if (password) {
                this.stronghold = await Hold.new("servers", password);
                
                const credentialsString = await this.stronghold?.get(currentServer);
                currentServerCredentials = credentialsString ? JSON.parse(credentialsString) as LoginResponse : null;

                document.getElementById("player-sidebar-avatar")?.setAttribute("src", atob(currentServerCredentials?.gamerpic ?? ""));
                await this.initializeAudioDevicesAndNetworkStream(this.store, currentServer ?? "", currentServerCredentials);
            }
        }

        
        // Render the dashboard
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
            await invoke("update_current_player").then(async () => {
                info("Updated current player");

                await this.changeNetworkStream(currentServer, credentials);
                
                await this.updateAudioDevice("OutputDevice");
                await this.updateAudioDevice("InputDevice");
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

                await invoke("change_audio_device", { device: device })
                    .then(() => {
                        info(`Audio device changed to ${device.name}`);
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

    async shutdown() {
        await invoke("stop_audio_device", { device: "InputDevice" });
        await invoke("stop_audio_device", { device: "OutputDevice" });
        await invoke("stop_network_stream");
    }
}