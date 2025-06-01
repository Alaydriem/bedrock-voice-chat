import { info, error, warn } from '@tauri-apps/plugin-log';
import { Store } from '@tauri-apps/plugin-store';
import { invoke } from "@tauri-apps/api/core";

// @ts-ignore
import murmurHash3 from "murmurHash3js";
import { mount } from 'svelte';
import ServerAvatar from '../../components/ServerAvatar.svelte';

import Hold from "./hold.ts";
import App from './app.js';

import { type LoginResponse } from "../bindings/LoginResponse";

declare global {
  interface Window {
    App: any;
  }
}

export default class Server extends App {

  private avatarSize: number = 32;

  constructor() {
      super();
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

    const stronghold = await Hold.new("servers"); 

    if (serverList.length === 1) {
      // Ping the server and check that we're authenticated
      const server = serverList[0].server;
      const credentialsString = await stronghold.get(server);
      const credentials: LoginResponse | null = credentialsString ? JSON.parse(credentialsString) as LoginResponse : null;

      if (!credentials) {
        error("No credentials found for server " + server + ", redirecting to login page");
        // If there's no credentials, redirect to the login page.
        window.location.href="/login";
        return;
      }

      await invoke("api_ping", { 
        endpoint: server,
        cert: credentials.certificate_ca,
        pem: credentials.certificate + credentials.certificate_key 
      })
        .then(async (response: any) => {
          window.location.href = "/dashboard";
        })
        .catch(async (e) => {
          error("Ping failed for server " + server + ": " + e);
          // If the ping fails, clear the credentials and redirect to the login page.
          stronghold.delete(server);
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

        const credentialsString = await stronghold.get(server);
        const credentials: LoginResponse | null = credentialsString ? JSON.parse(credentialsString) as LoginResponse : null;

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

        await invoke("api_ping", { 
          endpoint: server,
          cert: credentials.certificate_ca,
          pem: credentials.certificate + credentials.certificate_key 
        })
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
        });
      }
      
        // The ping all of them and show the ones that are connectable in color vs b/w
        // If the user clicks on a colored one
            // Go to the dashboard with the current server set
        
        // If the user clicks on a b/w one
            // Redirect to the login page with the domain pre-populated, then immediately jump to the login form
  }
}