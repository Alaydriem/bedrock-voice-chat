import { Store } from '@tauri-apps/plugin-store';
import { info } from '@tauri-apps/plugin-log';
import Analytics from '../analytics';
import { AppStore } from '../services/AppStore';
import type { SetupState } from '../../bindings/SetupState';

/**
 * The device screens that follow sign-in, and their persistence.
 *
 * Ordered: microphone, notifications, devices. `nextScreen` is the resume point.
 *
 * The stored state is a convenience, not the source of truth. Permissions and audio
 * hardware change underneath the app, so every screen re-checks the OS on arrival and
 * a completed one resolves without asking again. That is also why an install carried
 * forward from a build predating this key can simply run setup once more.
 */
export default class SetupFlow {
    static readonly STORE_KEY = 'setup_state';

    private static readonly ORDER: readonly (keyof SetupState)[] = [
        'microphone',
        'notifications',
        'devices',
    ];

    private store: Store | null = null;
    private state: SetupState = { microphone: false, notifications: false, devices: false };

    constructor(store: Store | null = null) {
        this.store = store;
    }

    /** Seed the state directly. For tests and for a caller that already read it. */
    hydrate(state: SetupState): void {
        this.state = { ...state };
    }

    async initialize(): Promise<void> {
        this.store ??= await AppStore.load();
        const stored = await this.store.get<SetupState>(SetupFlow.STORE_KEY);
        this.state = {
            microphone: stored?.microphone === true,
            notifications: stored?.notifications === true,
            devices: stored?.devices === true,
        };
        info(`Setup state: ${JSON.stringify(this.state)}`);
    }

    async completeStep(step: keyof SetupState): Promise<void> {
        this.state = { ...this.state, [step]: true };
        if (!this.store) return;
        await this.store.set(SetupFlow.STORE_KEY, this.state);
        await this.store.save();
    }

    nextScreen(): string | null {
        return SetupFlow.ORDER.find((step) => !this.state[step]) ?? null;
    }

    isComplete(): boolean {
        return this.nextScreen() === null;
    }

    currentState(): SetupState {
        return { ...this.state };
    }

    /**
     * Fired once, when the last screen is cleared. The event keeps its original name
     * despite the rename: it is the funnel's completion marker, and renaming it
     * orphans every historical event in PostHog.
     */
    reportCompletion(): void {
        Analytics.track('OnboardingCompleted');
    }
}
