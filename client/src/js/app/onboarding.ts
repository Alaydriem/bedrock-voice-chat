import App from "./app";
import { Store } from "@tauri-apps/plugin-store";
import type { OnboardingState } from "../bindings/OnboardingState";
import { info } from "@tauri-apps/plugin-log";

export default class Onboarding extends App {
    private store: Store | null = null;
    private state: OnboardingState;
    private readonly STORE_KEY = "onboarding_state";

    constructor(store: Store | null = null) {
        super();
        this.state = {
            welcome: false,
            microphone: false,
            notifications: false,
            devices: false,
        };
        this.store = store;
    }

    async initialize(): Promise<void> {
        if (this.store == null) {
            this.store = await Store.load("store.json", {
                autoSave: false,
                defaults: {}
            });
        }

        // Load existing onboarding state
        const storedState = await this.store.get<OnboardingState>(this.STORE_KEY);

        if (storedState) {
            this.state.welcome = storedState.welcome === true;
            this.state.microphone = storedState.microphone === true;
            this.state.notifications = storedState.notifications === true;
            this.state.devices = storedState.devices === true;

            await this.saveState();
        } else {
            info("No stored state found, initializing with defaults");
            await this.saveState();
        }
    }

    private async saveState(): Promise<void> {
        if (!this.store) return;
        await this.store.set(this.STORE_KEY, this.state);
        await this.store.save();
    }

    async completeStep(step: keyof OnboardingState): Promise<void> {
        this.state[step] = true;
        await this.saveState();
    }

    getNextStep(): string | null {
        if (!this.state.welcome) return "/onboarding/welcome";
        if (!this.state.microphone) return "/onboarding/microphone";
        if (!this.state.notifications) return "/onboarding/notifications";
        if (!this.state.devices) return "/onboarding/devices";
        return null;
    }

    isComplete(): boolean {
        return this.state.welcome &&
            this.state.microphone &&
            this.state.notifications &&
            this.state.devices;
    }

    getCurrentState(): OnboardingState {
        return { ...this.state };
    }

    async navigateToNext(): Promise<void> {
        const nextStep = this.getNextStep();
        if (nextStep) {
            window.location.href = nextStep;
        } else {
            window.location.href = "/dashboard";
        }
    }

    static async checkOnboardingRequired(): Promise<string | null> {
        const onboarding = new Onboarding();
        await onboarding.initialize();
        return onboarding.getNextStep();
    }
}
