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

export default class Keyring
{
    clientName: string;
    
    constructor(clientName: string) {
        this.clientName = clientName;
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
                key,
                value instanceof Uint8Array
                    ? Array.from(value)
                    : Array.from(new TextEncoder().encode(value))
            );
        }

        return await setPassword(
            key,
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
            return await getSecret(key).then((response) => {
                return new Uint8Array(response);
            })
        }

        return await getPassword(key);
    }

    async delete(key: string, type?: CredentialType | null): Promise<void> {
        if (type == null || type == undefined) {
            type = "Password";
        }

        if (type === "Secret") {
            return await deleteSecret(key);
        }

        return await deletePassword(key);
    }

    async has(key: string, type?: CredentialType | null): Promise<boolean> {
        if (type == null || type == undefined) {
            type = "Password";
        }

        if (type === "Secret") {
            return await hasSecret(key);
        }

        return await hasPassword(key);
    }
}