import { info, error } from '@tauri-apps/plugin-log';
import { writable, type Writable, type Readable } from 'svelte/store';

import BVCApp from './BVCApp.ts';

import { ServerHealthService } from './services/ServerHealthService';
import { ServerListStore } from './services/ServerListStore';
import type { ServerListEntry } from './services/ServerListStore';

declare global {
  interface Window {
    App: any;
  }
}

export default class Server extends BVCApp {
  private isRefreshingStore: Writable<boolean>;
  public readonly isRefreshing: Readable<boolean>;

  private serversStore: Writable<ServerListEntry[]>;
  public readonly servers: Readable<ServerListEntry[]>;

  private healthService: ServerHealthService;
  private serverListStore: ServerListStore;

  constructor() {
    super();
    this.isRefreshingStore = writable(false);
    this.isRefreshing = { subscribe: this.isRefreshingStore.subscribe };
    this.serversStore = writable([]);
    this.servers = { subscribe: this.serversStore.subscribe };
    this.healthService = new ServerHealthService();
    this.serverListStore = new ServerListStore();
  }

  async initialize() {
    if (await this.initializeDeepLinks()) {
      return;
    }

    const serverList = await this.serverListStore.getServerList();

    if (serverList.length === 0) {
      info("No servers found, redirecting to login page");
      window.location.href = "/login";
      return;
    }

    if (serverList.length === 1) {
      const server = serverList[0].server;

      try {
        const result = await this.healthService.check(server);
        switch (result.status) {
          case 'connect':
            window.location.href = "/dashboard";
            return;
          case 'reauth':
            window.location.href = `/login?reauth=true&server=${server}`;
            return;
          case 'missing':
            window.location.href = "/login";
            return;
          case 'version_mismatch':
            break;
        }
      } catch (e) {
        error(`Server unreachable or credentials invalid for ${server}: ${e}`);
        window.location.href = `/login?reauth=true&server=${server}`;
        return;
      }
    }

    // Reveal the selection UI only once we know we are staying on this page.
    // Every branch above either redirects or returns, keeping the preloader
    // overlay up so a single-server auto-redirect never flashes the grid.
    this.serversStore.set(serverList);
    this.showPreloader();
  }

  async refreshAll(): Promise<void> {
    this.isRefreshingStore.set(true);
    try {
      const serverList = await this.serverListStore.getServerList();
      if (serverList.length === 0) {
        info("Server list empty after refresh, redirecting to login page");
        window.location.href = "/login";
        return;
      }
      this.serversStore.set(serverList);
    } finally {
      this.isRefreshingStore.set(false);
    }
  }

  addServer(): void {
    window.location.href = "/login?addserver=true&return=/server";
  }

  showPreloader(): void {
    this.preloader();
  }
}
