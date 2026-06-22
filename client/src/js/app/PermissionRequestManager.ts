import { writable, type Writable, type Readable } from 'svelte/store';
import type { PermissionType } from 'tauri-plugin-audio-permissions';
import { checkPermissionStatus, requestPermissionWithTimeout } from './utils/permissionHelpers';
import { info, error as logError } from '@tauri-apps/plugin-log';

export type PermissionFlowState = 'idle' | 'requesting' | 'granted' | 'denied' | 'error';

export default class PermissionRequestManager {
	private readonly permissionType: PermissionType;
	private readonly requestTimeoutMs: number;
	private readonly pollIntervalMs: number;

	private stateStore: Writable<PermissionFlowState>;
	public readonly state: Readable<PermissionFlowState>;

	private currentState: PermissionFlowState = 'idle';
	private attemptId = 0;
	private pollTimer: number | null = null;
	private checking = false;
	private destroyed = false;
	private listenersAttached = false;

	private readonly onVisibilityChange: () => void;
	private readonly onFocus: () => void;

	constructor(
		permissionType: PermissionType,
		options: { requestTimeoutMs?: number; pollIntervalMs?: number } = {}
	) {
		this.permissionType = permissionType;
		this.requestTimeoutMs = options.requestTimeoutMs ?? 10000;
		this.pollIntervalMs = options.pollIntervalMs ?? 400;

		this.stateStore = writable<PermissionFlowState>('idle');
		this.state = { subscribe: this.stateStore.subscribe };

		this.onVisibilityChange = () => {
			if (document.visibilityState === 'visible') {
				void this.pollOnce();
				this.startPolling();
			} else {
				this.stopPolling();
			}
		};
		this.onFocus = () => { void this.pollOnce(); };
	}

	async start(): Promise<void> {
		if (this.destroyed) return;
		const granted = await this.safeCheck();
		if (this.destroyed) return;
		if (granted) {
			this.markGranted();
			return;
		}
		this.setState('idle');
		this.attachListeners();
		this.startPolling();
	}

	async requestPermission(): Promise<void> {
		if (this.isGranted) return;
		const myAttempt = ++this.attemptId;
		this.setState('requesting');

		try {
			const response = await requestPermissionWithTimeout(this.permissionType, this.requestTimeoutMs);
			if (myAttempt !== this.attemptId || this.destroyed || this.isGranted) return;
			if (response.granted) {
				this.markGranted();
			} else {
				this.setState('denied');
			}
		} catch (err) {
			if (myAttempt !== this.attemptId || this.destroyed || this.isGranted) return;
			const message = err instanceof Error ? err.message : String(err);
			if (message.toLowerCase().includes('timeout')) {
				info(`Permission request timed out for ${this.permissionType}; continuing to poll`);
				return;
			}
			logError(`Permission request failed for ${this.permissionType}: ${message}`);
			this.setState('error');
		}
	}

	cancel(): void {
		this.attemptId++;
		if (!this.isGranted) this.setState('idle');
	}

	destroy(): void {
		this.destroyed = true;
		this.stopPolling();
		this.detachListeners();
	}

	private get isGranted(): boolean {
		return this.currentState === 'granted';
	}

	private setState(next: PermissionFlowState): void {
		this.currentState = next;
		this.stateStore.set(next);
	}

	private markGranted(): void {
		this.stopPolling();
		this.detachListeners();
		this.setState('granted');
	}

	private attachListeners(): void {
		if (this.listenersAttached) return;
		window.addEventListener('focus', this.onFocus);
		document.addEventListener('visibilitychange', this.onVisibilityChange);
		this.listenersAttached = true;
	}

	private detachListeners(): void {
		if (!this.listenersAttached) return;
		window.removeEventListener('focus', this.onFocus);
		document.removeEventListener('visibilitychange', this.onVisibilityChange);
		this.listenersAttached = false;
	}

	private startPolling(): void {
		if (this.pollTimer !== null || this.destroyed) return;
		if (document.visibilityState === 'hidden') return;
		this.pollTimer = window.setInterval(() => { void this.pollOnce(); }, this.pollIntervalMs);
	}

	private stopPolling(): void {
		if (this.pollTimer !== null) {
			window.clearInterval(this.pollTimer);
			this.pollTimer = null;
		}
	}

	private async pollOnce(): Promise<void> {
		if (this.destroyed || this.checking || this.isGranted) return;
		this.checking = true;
		try {
			if (await this.safeCheck() && !this.destroyed && !this.isGranted) {
				this.markGranted();
			}
		} finally {
			this.checking = false;
		}
	}

	private async safeCheck(): Promise<boolean> {
		try {
			const response = await checkPermissionStatus(this.permissionType);
			return response.granted;
		} catch (err) {
			logError(`Permission check failed for ${this.permissionType}: ${String(err)}`);
			return false;
		}
	}
}
