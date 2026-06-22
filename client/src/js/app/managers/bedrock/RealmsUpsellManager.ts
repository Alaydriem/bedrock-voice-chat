import { writable, derived, type Writable, type Readable } from 'svelte/store';
import { info, error as logError } from '@tauri-apps/plugin-log';
import type { IapOffer } from '../../../bindings/IapOffer';
import type { BedrockManager } from './BedrockManager';

export class RealmsUpsellManager {
    private readonly bedrockManager: BedrockManager;

    private busyIdStore: Writable<string | null>;
    public readonly busyId: Readable<string | null>;
    private restoringStore: Writable<boolean>;
    public readonly restoring: Readable<boolean>;

    public readonly offers: Readable<IapOffer[]>;
    public readonly offersLoaded: Readable<boolean>;
    public readonly sortedOffers: Readable<IapOffer[]>;

    constructor(bedrockManager: BedrockManager) {
        this.bedrockManager = bedrockManager;
        this.offers = bedrockManager.offers;
        this.offersLoaded = bedrockManager.offersLoaded;

        this.busyIdStore = writable(null);
        this.busyId = { subscribe: this.busyIdStore.subscribe };
        this.restoringStore = writable(false);
        this.restoring = { subscribe: this.restoringStore.subscribe };

        this.sortedOffers = derived(this.offers, ($offers) =>
            [...$offers].sort((a, b) => Number(this.isAnnual(b)) - Number(this.isAnnual(a))),
        );
    }

    isAnnual(offer: IapOffer): boolean {
        return offer.product_id.toLowerCase().includes('annual');
    }

    cadence(offer: IapOffer): string {
        const id = offer.product_id.toLowerCase();
        if (id.includes('annual') || id.includes('year')) return '/ year';
        if (id.includes('month')) return '/ month';
        return '';
    }

    gradientStyle(offer: IapOffer): string {
        let hue = 0;
        for (let i = 0; i < offer.product_id.length; i++) {
            hue = (hue + offer.product_id.charCodeAt(i)) % 360;
        }
        return `background: linear-gradient(135deg, hsl(${hue}, 55%, 45%), hsl(${(hue + 120) % 360}, 45%, 35%))`;
    }

    async subscribe(productId: string): Promise<void> {
        this.busyIdStore.set(productId);
        try {
            await this.bedrockManager.purchase(productId);
            info(`Purchase flow completed for ${productId}`);
        } catch (e) {
            logError(`Purchase failed: ${e}`);
        } finally {
            this.busyIdStore.set(null);
        }
    }

    async restore(): Promise<void> {
        this.restoringStore.set(true);
        try {
            await this.bedrockManager.restorePurchases();
        } catch (e) {
            logError(`Restore failed: ${e}`);
        } finally {
            this.restoringStore.set(false);
        }
    }
}
