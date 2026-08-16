import { SettingsCatalogue } from "./SettingsCatalogue";

/** Where a settings pane lives. A child of the dashboard, which survives the nav. */
export class SettingsRoute {
    static readonly base = "/dashboard/settings";

    /** The screen settings is presented over, and the floor of the back stack. */
    static readonly root = "/dashboard";

    static href(pane: string): string {
        return `${this.base}/${pane}`;
    }

    /** The pane a path names, or the fallback for an unknown or unavailable one. */
    static paneOf(pathname: string, mobile = false): string {
        if (!pathname.startsWith(this.base)) return SettingsCatalogue.fallback;
        const id = pathname.slice(this.base.length).replace(/^\/+|\/+$/g, "");
        return SettingsCatalogue.find(id, mobile)?.id ?? SettingsCatalogue.fallback;
    }

    /** True when the path names a pane rather than the section list. */
    static isPane(pathname: string): boolean {
        if (!pathname.startsWith(this.base)) return false;
        return pathname.slice(this.base.length).replace(/^\/+|\/+$/g, "") !== "";
    }

    /**
     * How many screens sit between this path and the dashboard, which is the number of
     * entries that have to be popped to leave settings.
     *
     * A phone shows the section list and the pane as two screens. Desktop shows the
     * section nav beside the pane, so there is no list-only screen to stop at and both
     * paths are one level deep.
     */
    static depthOf(pathname: string, mobile: boolean): number {
        if (!pathname.startsWith(this.base)) return 0;
        if (!this.isPane(pathname)) return 1;
        return mobile ? 2 : 1;
    }

    /** The screen directly above this one. The dashboard is its own ceiling. */
    static parentOf(pathname: string, mobile: boolean): string {
        return this.depthOf(pathname, mobile) > 1 ? this.base : this.root;
    }
}
