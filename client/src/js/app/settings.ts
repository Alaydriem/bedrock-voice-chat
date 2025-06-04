import Hold from "./hold.ts";
import { Store } from '@tauri-apps/plugin-store';
import { info, error, warn } from '@tauri-apps/plugin-log';
import { invoke } from "@tauri-apps/api/core";
import Sidebar from "../../components/settings/Sidebar.svelte";
import App from './app.js';
import { onMount, mount } from "svelte";

declare global {
  interface Window {
    App: any;
  }
}

export default class Dashboard extends App {
    private store: Store | undefined;
    
    async initialize() {
        // Mount the sidebar
        const mainSidebarContainer = document.getElementById(
            "main-sidebar-container",
        );
        mount(Sidebar, {
            target: mainSidebarContainer!,
            props: {
                activePage: "audio.svelte"
            },
        });
    }
}