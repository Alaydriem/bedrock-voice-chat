import { SettingsRoute } from "./SettingsRoute";

/** A client-side navigation. `replaceState` is what makes a move not deepen the stack. */
type Navigate = (href: string, opts?: { replaceState?: boolean }) => Promise<void>;

/** A move through session history, by a negative number of entries. */
type Pop = (delta: number) => void;

/**
 * Where the back button goes inside settings.
 *
 * The platform back button is already correct: `WryActivity` climbs while the webview
 * can go back and hands the press to Android when it cannot. It only needs a history
 * stack that holds one entry per screen, which is what this produces — descending
 * pushes, a sibling move replaces, and going up pops rather than navigating.
 */
export class SettingsNavigation {
    /**
     * Whether the entries beneath the current screen are ones this session pushed.
     *
     * A boolean rather than a counter, because the system back pops without passing
     * through here: a counter would drift out of step and later pop past the dashboard.
     * The depth is read from the path on every call instead, which cannot drift.
     */
    private entered = false;

    /**
     * Where the gear lands on desktop, which has no section list to land on.
     *
     * Not the catalogue's fallback: that is where an unknown pane goes, and landing the
     * gear there would move it off the pane it has always opened.
     */
    private static readonly LANDING = "audio";

    constructor(
        private readonly navigate: Navigate,
        private readonly pop: Pop,
        private readonly mobile: () => boolean,
    ) {}

    /**
     * Opening settings.
     *
     * A phone lands on the section list; desktop has no list-only screen, so it lands on
     * a pane. Naming a pane on a phone pushes the list beneath it, so the shortcut that
     * saves a tap going in does not remove a level on the way out.
     */
    async open(pane?: string): Promise<void> {
        if (!this.mobile()) {
            await this.push(SettingsRoute.href(pane ?? SettingsNavigation.LANDING));
            return;
        }

        await this.push(SettingsRoute.base);
        if (pane) await this.push(SettingsRoute.href(pane));
    }

    /** Picking a pane: a descent to a second screen on a phone, a sibling move on desktop. */
    async select(pane: string): Promise<void> {
        const href = SettingsRoute.href(pane);
        if (this.mobile()) {
            await this.push(href);
            return;
        }

        await this.navigate(href, { replaceState: true });
    }

    /** One screen up from wherever the path says we are. */
    async up(pathname: string): Promise<void> {
        await this.unwind(1, SettingsRoute.parentOf(pathname, this.mobile()));
    }

    /** Out of settings, from any depth. */
    async exit(pathname: string): Promise<void> {
        await this.unwind(SettingsRoute.depthOf(pathname, this.mobile()), SettingsRoute.root);
    }

    private async push(href: string): Promise<void> {
        await this.navigate(href);
        this.entered = true;
    }

    /**
     * Climb by popping what was pushed.
     *
     * Reached without pushing anything — a cold launch into a settings URL, or the error
     * screen's deep link — there is no entry beneath to pop, and popping regardless
     * leaves the app. That case replaces the current entry with the destination, which
     * also puts the dashboard at the bottom of the stack where it belongs.
     */
    private async unwind(levels: number, fallback: string): Promise<void> {
        if (!this.entered || levels < 1) {
            await this.navigate(fallback, { replaceState: true });
            return;
        }

        if (fallback === SettingsRoute.root) this.entered = false;
        this.pop(-levels);
    }
}
