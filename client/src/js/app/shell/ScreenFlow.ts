import { writable, get, type Readable, type Writable } from 'svelte/store';
import { StepFlow } from '$radial/core/controllers/StepFlow';

export interface ScreenFlowOptions {
    readonly screens: readonly string[];
    readonly initial: string;
    /** Steps inside the stepped screen. Omit when a flow has none. */
    readonly steps?: number;
    readonly onScreen?: (name: string) => void;
}

/**
 * Which screen a single-page flow is showing, and which step within it.
 *
 * Screens are state rather than routes: one gating decision, one canvas mount, and
 * an entrance that is a class toggle instead of a page load.
 */
export default class ScreenFlow {
    private readonly options: ScreenFlowOptions;
    private readonly screenStore: Writable<string>;
    private readonly stepStore: Writable<number>;

    public readonly screen: Readable<string>;
    public readonly step: Readable<number>;
    public readonly total: number;

    constructor(options: ScreenFlowOptions) {
        this.options = options;
        this.total = options.steps ?? 1;
        this.screenStore = writable(options.initial);
        this.stepStore = writable(1);
        this.screen = { subscribe: this.screenStore.subscribe };
        this.step = { subscribe: this.stepStore.subscribe };
    }

    public go(name: string): void {
        if (!this.options.screens.includes(name)) return;
        if (get(this.screenStore) === name) return;
        this.screenStore.set(name);
        this.stepStore.set(1);
        this.options.onScreen?.(name);
    }

    public goStep(step: number): void {
        this.stepStore.set(Math.max(1, Math.min(this.total, step)));
    }

    public nextStep(): void {
        this.goStep(get(this.stepStore) + 1);
    }

    public backStep(): void {
        this.goStep(get(this.stepStore) - 1);
    }

    public isLastStep(): boolean {
        return get(this.stepStore) >= this.total;
    }

    /**
     * Replay the entry stagger. Delegates to the kit, whose implementation removes
     * the animation, forces a layout read and restores it — a reflow that is
     * load-bearing rather than superstition.
     */
    public restage(root: ParentNode | null): void {
        if (root) StepFlow.restartStagger(root);
    }
}
