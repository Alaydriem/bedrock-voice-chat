import { invoke } from "@tauri-apps/api/core";
import { error as logError } from "@tauri-apps/plugin-log";
import { writable, type Readable, type Writable } from "svelte/store";
import type { Store } from "@tauri-apps/plugin-store";
import type { LanguagePack } from "../../../bindings/LanguagePack";
import { I18n } from "$lib/i18n";

const KEY = "locale";
const AUTO = "auto";

/**
 * Chooses which language pack the application runs in.
 *
 * A failure here never propagates: message ids are English, so the worst outcome of a
 * missing or unreadable pack is an untranslated app rather than a broken one.
 */
export default class LocaleManager {
    private localesStore: Writable<string[]>;
    private activeStore: Writable<string>;

    public readonly locales: Readable<string[]>;
    public readonly active: Readable<string>;

    constructor(private store: Store) {
        this.localesStore = writable([]);
        this.activeStore = writable(AUTO);
        this.locales = { subscribe: this.localesStore.subscribe };
        this.active = { subscribe: this.activeStore.subscribe };
    }

    async initialize(): Promise<void> {
        try {
            this.localesStore.set(await invoke<string[]>("i18n_locales"));
        } catch (e: unknown) {
            await logError(`LocaleManager: could not list locales: ${LocaleManager.reason(e)}`);
            this.localesStore.set([]);
        }

        const stored = ((await this.store.get(KEY)) as string | null) ?? AUTO;
        this.activeStore.set(stored);
        await this.apply(stored);
    }

    async choose(locale: string): Promise<void> {
        this.activeStore.set(locale);
        await this.store.set(KEY, locale);
        await this.store.save();
        await this.apply(locale);
    }

    private async apply(locale: string): Promise<void> {
        const requested = locale === AUTO ? navigator.language : locale;

        try {
            I18n.adopt(await invoke<LanguagePack | null>("i18n_load", { requested }));
        } catch (e: unknown) {
            await logError(
                `LocaleManager: could not load locale ${requested}: ${LocaleManager.reason(e)}`,
            );
            I18n.adopt(null);
        }
    }

    private static reason(e: unknown): string {
        return e instanceof Error ? e.message : String(e);
    }
}
