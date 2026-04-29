import { info, error, warn } from '@tauri-apps/plugin-log';
import { invoke } from "@tauri-apps/api/core";

// @ts-ignore
import murmurHash3 from "murmurhash3js";
import { mount } from 'svelte';
import ServerCard from '../../components/ServerCard.svelte';

import BVCApp from './BVCApp.ts';

import { type LoginResponse } from "../bindings/LoginResponse";

declare global {
  interface Window {
    App: any;
  }
}

export default class Server extends BVCApp {

  constructor() {
      super();
  }

  async initialize() {
    await this.initializeDeepLinks();

    const store = await this.getStore();
    let serverList = await store.get("server_list") as Array<{ server: string, player: string }>;

    if (serverList == null || serverList.length === 0) {
      info("No servers found, redirecting to login page");
      window.location.href="/login";
      return;
    }

    if (serverList.length === 1) {
      const server = serverList[0].server;

      let credentials: LoginResponse;
      try {
        credentials = await invoke<LoginResponse>("get_credentials", { server });
      } catch (e) {
        error("No credentials found for server " + server + ", redirecting to login page");
        window.location.href="/login";
        return;
      }

      try {
        const expired = await invoke<boolean>("is_certificate_expired", { server });
        if (expired) {
          info("Certificate expired for server " + server + ", redirecting to login");
          window.location.href = "/login?reauth=true&server=" + server;
          return;
        }
      } catch (e) {
        warn("Could not check certificate expiry for " + server + ": " + e);
      }

      await invoke("api_initialize_client", {
        endpoint: server,
        cert: credentials.certificate_ca,
        pem: credentials.certificate + credentials.certificate_key
      });

      try {
        await invoke("refresh_server_state", { server });
        window.location.href = "/dashboard";
      } catch (e) {
        error("Server unreachable or credentials invalid for " + server + ": " + e);
        window.location.href = "/login?reauth=true&server=" + server;
      }
      return;
    }

    await this.renderServerList();
  }

  async renderServerList(): Promise<void> {
    const store = await this.getStore();
    const serverList = await store.get("server_list") as Array<{ server: string, player: string, game?: string }> | null;

    if (!serverList || serverList.length === 0) {
      info("Server list empty after refresh, redirecting to login page");
      window.location.href = "/login";
      return;
    }

    const container = document.getElementById('server-avatar-container');
    if (!container) {
      error("Container for server avatars not found");
      return;
    }

    container.innerHTML = '';

    serverList.forEach(({ server }) => {
      const bytes = new TextEncoder().encode(server);
      const byteString = Array.from(bytes)
        .map((byte) => String.fromCharCode(byte))
        .join('');
      const hash = murmurHash3.x86.hash128(byteString);

      mount(ServerCard, {
        target: container,
        props: {
          id: hash,
          server: server,
          onRemoved: () => { this.renderServerList(); },
        },
      });
    });
  }
}
