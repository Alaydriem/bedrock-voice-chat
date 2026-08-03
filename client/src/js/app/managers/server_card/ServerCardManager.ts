import { writable, derived, get, type Writable, type Readable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { error as logError, info } from '@tauri-apps/plugin-log';
import Analytics from '../../analytics';
import ImageCacheOptions from '../../components/imageCacheOptions';
import type { ServerCardStatus } from './ServerCardStatus';
import type { ServerCardButtonState } from './ServerCardButtonState';
import type { ServerCardBadgeState } from './ServerCardBadgeState';
import type { ServerCardManagerArgs } from './ServerCardManagerArgs';
import type { ServerCardManagerDeps } from './ServerCardManagerDeps';
import type { NextAction } from './NextAction';

export class ServerCardManager {
    private static readonly IMAGE_CACHE_TTL_SECONDS = 60 * 60 * 24 * 7;

    private readonly id: string;
    private readonly server: string;
    private readonly deps: ServerCardManagerDeps;

    private statusStore: Writable<ServerCardStatus>;
    private clientTooOldStore: Writable<boolean>;
    private serverVersionStore: Writable<string>;
    private clientVersionStore: Writable<string>;
    private canvasImageStore: Writable<string>;
    private avatarImageStore: Writable<string>;

    public readonly status: Readable<ServerCardStatus>;
    public readonly canvasImage: Readable<string>;
    public readonly avatarImage: Readable<string>;
    public readonly button: Readable<ServerCardButtonState>;
    public readonly badge: Readable<ServerCardBadgeState>;
    public readonly displayHost: string;
    public readonly gradientStyle: string;

    constructor(args: ServerCardManagerArgs, deps: ServerCardManagerDeps) {
        this.id = args.id;
        this.server = args.server;
        this.deps = deps;

        this.statusStore = writable<ServerCardStatus>('checking');
        this.clientTooOldStore = writable(false);
        this.serverVersionStore = writable('');
        this.clientVersionStore = writable('');
        this.canvasImageStore = writable('');
        this.avatarImageStore = writable('');

        this.status = { subscribe: this.statusStore.subscribe };
        this.canvasImage = { subscribe: this.canvasImageStore.subscribe };
        this.avatarImage = { subscribe: this.avatarImageStore.subscribe };

        this.button = derived(
            [this.statusStore, this.clientTooOldStore, this.serverVersionStore, this.clientVersionStore],
            ([$status, $tooOld, $serverVersion, $clientVersion]) =>
                ServerCardManager.deriveButton($status, $tooOld, $serverVersion, $clientVersion),
        );

        this.badge = derived(this.statusStore, ($status) => ServerCardManager.deriveBadge($status));

        this.displayHost = this.server.replace(/^https?:\/\//, '');
        this.gradientStyle = ServerCardManager.deriveGradientStyle(this.id);
    }

    async initialize(): Promise<void> {
        this.loadImages();
        await this.refresh();
    }

    async refresh(): Promise<void> {
        this.statusStore.set('checking');
        try {
            const result = await this.deps.health.check(this.server);
            const cardStatus: ServerCardStatus = result.status === 'missing' ? 'reauth' : result.status;
            this.statusStore.set(cardStatus);
            this.clientTooOldStore.set(result.clientTooOld);
            this.serverVersionStore.set(result.serverVersion);
            this.clientVersionStore.set(result.clientVersion);
        } catch (e) {
            logError(`Failed to check server ${this.server}: ${e}`);
            this.statusStore.set('reauth');
        }
    }

    async handleAction(): Promise<NextAction> {
        const status = get(this.statusStore);
        if (status === 'version_mismatch' || status === 'checking') {
            return { kind: 'none' };
        }

        if (status === 'connect') {
            const entry = await this.deps.serverList.findEntry(this.server);
            if (entry) {
                await this.deps.serverList.setCurrent({
                    server: this.server,
                    player: entry.player,
                    game: entry.game ?? 'minecraft',
                });
            }
            Analytics.track('ServerSelected');
            return { kind: 'navigate', href: `/dashboard?server=${this.server}` };
        }

        return { kind: 'navigate', href: `/login?reauth=true&server=${this.server}` };
    }

    async remove(): Promise<NextAction> {
        try {
            await invoke('delete_credentials', { server: this.server }).catch((e) => {
                info(`delete_credentials failed for ${this.server}: ${e}`);
            });

            const remaining = await this.deps.serverList.removeServer(this.server);

            if (remaining.length === 0) {
                return { kind: 'navigate', href: '/login' };
            }
            return { kind: 'none' };
        } catch (e) {
            logError(`Failed to remove server ${this.server}: ${e}`);
            return { kind: 'none' };
        }
    }

    private loadImages(): void {
        const ttl = ServerCardManager.IMAGE_CACHE_TTL_SECONDS;
        this.deps.imageCache
            .getImage(new ImageCacheOptions(`${this.server}/assets/canvas.png`, ttl))
            .then((url) => this.canvasImageStore.set(url))
            .catch(() => this.canvasImageStore.set(''));
        this.deps.imageCache
            .getImage(new ImageCacheOptions(`${this.server}/assets/avatar.png`, ttl))
            .then((url) => this.avatarImageStore.set(url))
            .catch(() => this.avatarImageStore.set(''));
    }

    private static deriveBadge(status: ServerCardStatus): ServerCardBadgeState {
        switch (status) {
            case 'connect':
                return { label: 'Online', classes: 'bg-success/80' };
            case 'reauth':
                return { label: 'Auth required', classes: 'bg-error/80' };
            case 'version_mismatch':
                return { label: 'Outdated', classes: 'bg-warning/80' };
            case 'checking':
            default:
                return { label: 'Checking…', classes: 'bg-slate-500/80' };
        }
    }

    private static deriveButton(
        status: ServerCardStatus,
        clientTooOld: boolean,
        serverVersion: string,
        clientVersion: string,
    ): ServerCardButtonState {
        switch (status) {
            case 'connect':
                return {
                    label: 'Connect',
                    classes: 'bg-success hover:bg-success-focus text-white',
                    disabled: false,
                };
            case 'reauth':
                return {
                    label: 'Re-authenticate',
                    classes: 'bg-error hover:bg-error-focus text-white',
                    disabled: false,
                };
            case 'version_mismatch':
                return {
                    label: clientTooOld
                        ? `Update Client (${clientVersion} → ${serverVersion})`
                        : 'Server Outdated',
                    classes: 'bg-warning text-slate-800 cursor-not-allowed',
                    disabled: true,
                    title: clientTooOld ? undefined : 'Server is running an older protocol',
                };
            case 'checking':
            default:
                return {
                    label: 'Checking…',
                    classes: 'bg-slate-200 text-slate-500 dark:bg-navy-600 dark:text-navy-300 cursor-wait',
                    disabled: true,
                };
        }
    }

    private static deriveGradientStyle(id: string): string {
        let total = 0;
        for (let i = 0; i < id.length; i++) {
            total = (total + id.charCodeAt(i)) % 360;
        }
        const hue = total;
        return `background: linear-gradient(135deg, hsl(${hue}, 55%, 45%), hsl(${(hue + 120) % 360}, 45%, 35%))`;
    }
}
