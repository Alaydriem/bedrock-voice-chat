import { info, error, warn } from '@tauri-apps/plugin-log';
import { Store } from '@tauri-apps/plugin-store';
import { invoke } from "@tauri-apps/api/core";

// @ts-ignore
import murmurHash3 from "murmurhash3js";
import { mount } from 'svelte';
import ServerAvatar from '../../components/ServerAvatar.svelte';

import Keyring from "./keyring.ts";
import App from './app.js';

import { type LoginResponse } from "../bindings/LoginResponse";

// Utility function to get all keys from a type at runtime
function getLoginResponseKeys(): (keyof LoginResponse)[] {
  // Create a sample object that satisfies the LoginResponse type
  const sample: Record<keyof LoginResponse, any> = {
    gamerpic: null,
    gamertag: null,
    keypair: null,
    signature: null,
    certificate: null,
    certificate_key: null,
    certificate_ca: null,
    quic_connect_string: null
  };
  return Object.keys(sample) as (keyof LoginResponse)[];
}

declare global {
  interface Window {
    App: any;
  }
}

export default class Server extends App {

  private avatarSize: number = 32;
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
    // Get keys from the LoginResponse type dynamically
    const keys = getLoginResponseKeys();

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

  async deleteCredentials(server: string): Promise<void> {
    const keys = getLoginResponseKeys();

    if (!this.keyring) {
      throw new Error("Keyring not initialized");
    }

    for (const key of keys) {
      await this.keyring.delete(key);
    }
  }

  async initialize() {
    const store = await Store.load('store.json', { autoSave: false });
    let serverList = await store.get("server_list") as Array<{ server: string, player: string }>;

    // If there are none, redirect to the login page.
    if (serverList == null || serverList.length === 0) {
      info("No servers found, redirecting to login page");
      window.location.href="/login";
      return;
    }

    this.keyring = await Keyring.new("servers");

    if (serverList.length === 1) {
      // Ping the server and check that we're authenticated
      const server = serverList[0].server;
      await this.keyring.setServer(server);

      const credentials = await this.getCredentials();

      if (!credentials) {
        error("No credentials found for server " + server + ", redirecting to login page");
        // If there's no credentials, redirect to the login page.
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
          // If the ping fails, clear the credentials and redirect to the login page.
          this.deleteCredentials(server);
          // Delete the item from server_list
          serverList = serverList.filter((item) => item.server !== server);
          await store.set("server_list", serverList);
          await store.save();
          window.location.href = "/login?reauth=true&server=" + server;
        });
    } else {
      // If there's more than one server
      const container = document.getElementById('server-avatar-container');
      if (!container) {
        console.error("Container for server avatars not found");
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
        document.getElementById(hash)?.remove();
        mount(ServerAvatar, {
          target: container,
          props: {
            id: hash,
            server: server
          },
        });
      });

      serverList.forEach(async ({ server }) => {
        const bytes = new TextEncoder().encode(server);
        const byteString = Array.from(bytes)
          .map((byte) => String.fromCharCode(byte))
          .join('');
        const hash = murmurHash3.x86.hash128(byteString);
        const card = document.getElementById(hash);
        const button = card?.querySelector("button");
        if (!button) { return; }

        // Create a separate keyring instance for each server
        const serverKeyring = await Keyring.new("servers");
        await serverKeyring.setServer(server);
        
        // Create a temporary server instance with the server-specific keyring
        const tempServer = new Server();
        await tempServer.setKeyring(serverKeyring, server);
        const credentials = await tempServer.getCredentials();
        if (!credentials) {
          error("No credentials found for server " + server + ", prompting for re-authentication");
          button.removeAttribute("disabled");
          button.querySelector(".spinner")?.remove();
          button.classList.remove("text-grey");
          button.classList.remove("bg-primary");
          button.classList.add("bg-error");
          button.classList.add("text-white");
          const message = button.querySelector("#message");
          if (message) {
            message.innerHTML = "Re-authenticate";
          }

          button.addEventListener("click", async () => {
            // If there's no credentials, redirect to the login page.
            window.location.href="/login?reauth=true&server=" + server;
          });
          return;
        }

        const cert = typeof credentials.certificate_ca === 'string' ? credentials.certificate_ca : new TextDecoder().decode(credentials.certificate_ca);
        const certKeyStr = typeof credentials.certificate_key === 'string' ? credentials.certificate_key : new TextDecoder().decode(credentials.certificate_key);
        const certStr = typeof credentials.certificate === 'string' ? credentials.certificate : new TextDecoder().decode(credentials.certificate);
        const pem = certStr + certKeyStr;

        console.log("Initializing client for server " + server);
        console.log(cert);
        console.log(pem);
        await invoke("api_initialize_client", {
          endpoint: server,
          cert: cert,
          pem: pem
        }).then(async () => {
          await invoke("api_ping")
            .then(async (response: any) => {
              button.removeAttribute("disabled");
              button.querySelector(".spinner")?.remove();
              const message = button.querySelector("#message");
              button.classList.remove("text-grey");
              button.classList.remove("bg-primary");
              button.classList.add("bg-success");
              button.classList.add("text-white");
              if (message) {
                message.innerHTML = "Connect!";
              }

              button.addEventListener("click", async () => {
                // If there's no credentials, redirect to the login page.
                store.set("current_server", server);
                await store.save();
                window.location.href="/dashboard?server=" + server;
              });
            })
            .catch(async (e) => {
              error("Ping failed for server " + server + ": " + e);
              button.removeAttribute("disabled");
              button.querySelector(".spinner")?.remove();
              button.classList.remove("text-grey");
              button.classList.remove("bg-primary");
              button.classList.add("bg-error");
              button.classList.add("text-white");
              const message = button.querySelector("#message");
              if (message) {
                message.innerHTML = "Re-authenticate";
              }

              button.addEventListener("click", async () => {
                // If there's no credentials, redirect to the login page.
                window.location.href="/login?reauth=true&server=" + server;
              });
            });
          }).catch((e) => {
            error("Failed to initialize client for server " + server + ": " + e);
          });
        });
      }

        // The ping all of them and show the ones that are connectable in color vs b/w
        // If the user clicks on a colored one
            // Go to the dashboard with the current server set

        // If the user clicks on a b/w one
            // Redirect to the login page with the domain pre-populated, then immediately jump to the login form
  }
}