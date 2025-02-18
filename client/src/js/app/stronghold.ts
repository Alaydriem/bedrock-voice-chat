import { Client, Stronghold } from '@tauri-apps/plugin-stronghold';
import { appDataDir } from '@tauri-apps/api/path';
import { info, error, warn } from '@tauri-apps/plugin-log';
import { invoke } from "@tauri-apps/api/core";

// @todo: Scrap all of this in favor of native keyrings
// hwchen/keyring-rs can do Windows, and iOS, but not yet Android
// Kotlin ffi bindings are a separate option.
export default class Hold
{
    clientName: string;
    vaultPassword: string;
    stronghold: Stronghold | null;
    client: Client | null;
    
    constructor(clientName: string, vaultPassword: string, stronghhold: Stronghold | null, client: Client | null) {
        this.clientName = clientName;
        this.vaultPassword = vaultPassword;

        this.stronghold = stronghhold;
        this.client = client;
    }

    // Creates a new instance of hold configured for the environment
    static async new(clientName: string, vaultPassword: string): Promise<Hold> {
        // Stronghold is _incredibly_ poor performing in cargo run // debug release
        // Use localStorage instead
        // DO NOT RUN DEBUG BUILDS AS SHIPPABLE APPLICATIONS
        if (await Hold.is_dev()) {
            return new Hold(clientName, vaultPassword, null, null);
        }

        const vaultPath = `${await appDataDir()}/vault.hold`;
        const stronghold = await Stronghold.load(vaultPath, vaultPassword);
      
        let client: Client;
        try {
          client = await stronghold.loadClient(clientName);
        } catch {
          client = await stronghold.createClient(clientName);
        }
      
        return new Hold(clientName, vaultPassword, stronghold, client);
    }

    async insert(key: string, value: string): Promise<void> {
        if (await Hold.is_dev()) {
            return new Promise((resolve, reject) => {
                resolve(localStorage.setItem(key, value));
            });
        };

        const data = Array.from(new TextEncoder().encode(value));
        return await this.client!.getStore().insert(key, data);
    }

    async get(key: string): Promise<string> {
        if (await Hold.is_dev()) {
            return new Promise((resolve, reject) => {
                const result = localStorage.getItem(key);
                if (result === null) {
                    reject(`No data found for key: ${key}`);
                }

                return resolve(result as string);
            });
        };

        const data = await this.client!.getStore().get(key);
        if (data === null) {
            throw new Error(`No data found for key: ${key}`);
        }
        return new TextDecoder().decode(new Uint8Array(data));
    }

    async delete(key: string): Promise<Uint8Array | null> {
        if (await Hold.is_dev()) {
            return new Promise((resolve, reject) => {
                localStorage.delete(key);
                resolve(null);
            });
        };
        return await this.client!.getStore().remove(key);
    }
    
    async commit(): Promise<void> {
        if (await Hold.is_dev()) {
            return new Promise((resolve, reject) => {
                return resolve();
            });
        };

        await this.stronghold!.save();
    }

    static async is_dev(): Promise<boolean> {
        return await invoke("get_variant") == "dev";
    }
}