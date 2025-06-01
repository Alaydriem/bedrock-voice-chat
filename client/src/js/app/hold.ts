import { invoke } from "@tauri-apps/api/core";

// @todo: Scrap all of this in favor of native keyrings
// hwchen/keyring-rs can do Windows, and iOS, but not yet Android
// Kotlin ffi bindings are a separate option.
export default class Hold
{
    clientName: string;
    
    constructor(clientName: string) {
        this.clientName = clientName;
    }

    // Creates a new instance of hold configured for the environment
    static async new(clientName: string): Promise<Hold> {
        // DO NOT RUN DEBUG BUILDS AS SHIPPABLE APPLICATIONS
        if (await Hold.is_dev()) {
            return new Hold(clientName);
        }
      
        return new Hold(clientName);
    }

    async insert(key: string, value: string): Promise<void> {
        if (await Hold.is_dev()) {
            return new Promise((resolve, reject) => {
                resolve(localStorage.setItem(key, value));
            });
        };
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

        return "";
    }

    async delete(key: string): Promise<Uint8Array | null> {
        if (await Hold.is_dev()) {
            return new Promise((resolve, reject) => {
                localStorage.removeItem(key);
                resolve(null);
            });
        };

        return null;
    }
    
    async commit(): Promise<void> {
        if (await Hold.is_dev()) {
            return new Promise((resolve, reject) => {
                return resolve();
            });
        };

    }

    static async is_dev(): Promise<boolean> {
        return true;
        //return await invoke("get_variant") == "dev";
    }
}