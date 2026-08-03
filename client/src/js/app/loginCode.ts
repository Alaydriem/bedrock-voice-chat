import { info, error as logError } from '@tauri-apps/plugin-log';
import { invoke } from '@tauri-apps/api/core';
import { writable, type Readable, type Writable } from 'svelte/store';
import BVCApp from './BVCApp.ts';
import type { LoginResponse } from '../bindings/LoginResponse';
import type { Game } from '../bindings/Game';
import type { ServerListEntry } from '../bindings/ServerListEntry';
import { ServerListStore } from './services/ServerListStore';

export interface CodeLoginInput {
    readonly code: string;
}

/**
 * The sign-in-code flow: the player types the code they were given, and nothing else.
 *
 * The gamertag and the game arrive on the response, because the server resolves both from
 * the code. That is what the stored identity is written from — the server is the authority
 * on which game an account belongs to, not the person signing in.
 *
 * Values arrive as arguments rather than being read back out of the DOM, so the
 * screen owns its inputs and this owns the attempt.
 */
export default class LoginCode extends BVCApp {
    private serverUrl = '';
    private readonly errorStore: Writable<string>;
    private readonly isSubmittingStore: Writable<boolean>;

    public readonly error: Readable<string>;
    public readonly isSubmitting: Readable<boolean>;

    constructor() {
        super();
        this.errorStore = writable('');
        this.isSubmittingStore = writable(false);
        this.error = { subscribe: this.errorStore.subscribe };
        this.isSubmitting = { subscribe: this.isSubmittingStore.subscribe };
    }

    setServer(server: string): void {
        this.serverUrl = server;
    }

    async submit(input: CodeLoginInput): Promise<void> {
        this.errorStore.set('');

        const code = input.code.trim().toUpperCase();
        if (!code) {
            this.errorStore.set('Please enter your code.');
            return;
        }

        this.isSubmittingStore.set(true);
        try {
            info(`Attempting code login to ${this.serverUrl}`);
            const response = await invoke<LoginResponse>('code_login', {
                server: this.serverUrl,
                code,
            });

            // A server too old to report the game predates this being anything but
            // Minecraft, which is what the picker this replaced always defaulted to.
            const game: Game = response.game ?? 'minecraft';

            const store = await this.getStore();
            const [rawServerList] = await Promise.all([
                store.get('server_list') as Promise<ServerListEntry[] | null>,
                store.set('current_server', this.serverUrl),
                store.set('current_player', response.gamertag),
                store.set('active_game', game),
            ]);

            const serverList = rawServerList || [];
            const existing = serverList.find((s) => s.server === this.serverUrl);
            if (existing) {
                existing.game = game;
            } else {
                serverList.push({
                    server: this.serverUrl,
                    player: response.gamertag,
                    game,
                });
            }
            await store.set('server_list', serverList);
            await store.save();
            ServerListStore.mirrorServerCount(serverList);

            info('Code login successful, continuing to setup');
            window.location.href = '/setup';
        } catch (e) {
            logError(`Code login failed: ${String(e)}`);
            this.errorStore.set('That code was not accepted. Check it and try again.');
            this.isSubmittingStore.set(false);
        }
    }
}
