import { info, error } from '@tauri-apps/plugin-log';
import { invoke } from "@tauri-apps/api/core";

// @ts-ignore
import murmurHash3 from "murmurhash3js";
import { mount } from 'svelte';
import ServerAvatar from '../../components/ServerAvatar.svelte';

import Keyring from "./keyring.ts";
import BVCApp from './BVCApp.ts';

import { type LoginResponse } from "../bindings/LoginResponse";

declare global {
  interface Window {
    App: any;
  }
}

export default class Server extends BVCApp {

  private keyring: Keyring | undefined;

  constructor() {
      super();
  }

  async setKeyring(keyring: Keyring, server: string) {
    this.keyring = keyring;
    await this.keyring.setServer(server);
  }

  async getCredentials(): Promise<LoginResponse | null> {
    const response: LoginResponse = {} as LoginResponse;
    const keys: (keyof LoginResponse)[] = [
      'gamerpic', 'gamertag', 'keypair', 'signature',
      'certificate', 'certificate_key', 'certificate_ca', 'quic_connect_string'
    ];

    if (!this.keyring) {
      throw new Error("Keyring not initialized");
    }

    for (const key of keys) {
      const storedValue = await this.keyring.get(key);
      if (key === "keypair" || key === "signature") {
        let valueStr: string;
        if (typeof storedValue === "string") {
          valueStr = storedValue;
        } else if (storedValue instanceof Uint8Array) {
          valueStr = new TextDecoder().decode(storedValue);
        } else {
          valueStr = "";
        }
        (response as any)[key] = JSON.parse(valueStr);
      } else {
        (response as any)[key] = storedValue;
      }
    }

    return response;
  }

  async initialize() {
    await this.initializeDeepLinks();

    const store = await this.getStore();
    let serverList = await store.get("server_list") as Array<{ server: string, player: string }>;

    // If there are none, redirect to the login page.
    if (serverList == null || serverList.length === 0) {
      info("No servers found, redirecting to login page");
      window.location.href="/login";
      return;
    }

    if (serverList.length === 1) {
      // Single server: quick path using default client
      const server = serverList[0].server;
      this.keyring = await Keyring.new("servers");
      await this.keyring.setServer(server);

      const credentials = await this.getCredentials();

      if (!credentials) {
        error("No credentials found for server " + server + ", redirecting to login page");
        window.location.href="/login";
        return;
      }

      await invoke("api_initialize_client", {
        endpoint: server,
        cert: credentials.certificate_ca,
        pem: credentials.certificate + credentials.certificate_key
      });

      await invoke("api_ping")
        .then(async (response: any) => {
          window.location.href = "/dashboard";
        })
        .catch(async (e) => {
          error("Ping failed for server " + server + ": " + e);
          window.location.href = "/login?reauth=true&server=" + server;
        });
    } else {
      // Multiple servers: mount components, they handle themselves
      const container = document.getElementById('server-avatar-container');
      if (!container) {
        error("Container for server avatars not found");
        return;
      }

      container.innerHTML = '';

      // Loop through the server list and create avatars
      serverList.forEach(({ server }) => {
        const bytes = new TextEncoder().encode(server);
        const byteString = Array.from(bytes)
          .map((byte) => String.fromCharCode(byte))
          .join('');
        const hash = murmurHash3.x86.hash128(byteString);

        mount(ServerAvatar, {
          target: container,
          props: {
            id: hash,
            server: server
          },
        });
      });
    }
  }
}