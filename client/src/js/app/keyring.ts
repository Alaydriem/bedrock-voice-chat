import {
  initializeKeyring,
  setPassword,
  getPassword,
  deletePassword,
  hasPassword,
  setSecret,
  getSecret,
  deleteSecret,
  hasSecret,
} from 'tauri-plugin-keyring'
import type { CredentialType, CredentialValue } from 'tauri-plugin-keyring'
import { getIdentifier } from '@tauri-apps/api/app';
import { info } from '@tauri-apps/plugin-log';

export default class Keyring
{
    clientName: string;
    
    server: string;

    constructor(clientName: string) {
        this.clientName = clientName;
        this.server = '';
    }

    async setServer(server: string) {
        this.server = server;
    }

    // Creates a new instance of Keyring configured for the environment
    static async new(clientName: string): Promise<Keyring> {
        const identifier = await getIdentifier();
        try {
            await initializeKeyring(identifier + '-' + clientName);
        } catch (e) {
            // Ignore already initialized error
        }
        return new Keyring(clientName);
    }

    async insert(key: string, value: string|Uint8Array, type?: CredentialType | null): Promise<void> {
        if (type == null || type == undefined) {
            type = "Password";
        }
        
        if (type === "Secret") {
            return await setSecret(
                btoa(this.server + "/" + key),
                value instanceof Uint8Array
                    ? Array.from(value)
                    : Array.from(new TextEncoder().encode(value))
            );
        }

        return await setPassword(
            btoa(this.server + "/" + key),
            value instanceof Uint8Array
                ? new TextDecoder().decode(value)
                : value
        );
    }

    async get(key: string, type?: CredentialType | null): Promise<string|Uint8Array> {
        if (type == null || type == undefined) {
            type = "Password";
        }

        if (type === "Secret") {
            return await getSecret(btoa(this.server + "/" + key)).then((response) => {
                return new Uint8Array(response);
            })
        }

        return await getPassword(btoa(this.server + "/" + key));
    }

    async delete(key: string, type?: CredentialType | null): Promise<void> {
        if (type == null || type == undefined) {
            type = "Password";
        }

        if (type === "Secret") {
            return await deleteSecret(btoa(this.server + "/" + key));
        }

        return await deletePassword(btoa(this.server + "/" + key));
    }

    async has(key: string, type?: CredentialType | null): Promise<boolean> {
        if (type == null || type == undefined) {
            type = "Password";
        }

        if (type === "Secret") {
            return await hasSecret(btoa(this.server + "/" + key));
        }

        return await hasPassword(btoa(this.server + "/" + key));
    }
}