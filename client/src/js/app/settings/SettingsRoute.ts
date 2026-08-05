import { SettingsCatalogue } from "./SettingsCatalogue";

/** Where a settings pane lives. A child of the dashboard, which survives the nav. */
export class SettingsRoute {
    static readonly base = "/dashboard/settings";

    static href(pane: string): string {
        return `${this.base}/${pane}`;
    }

    /** The pane a path names, or the fallback for an unknown or unavailable one. */
    static paneOf(pathname: string, mobile = false): string {
        if (!pathname.startsWith(this.base)) return SettingsCatalogue.fallback;
        const id = pathname.slice(this.base.length).replace(/^\/+|\/+$/g, "");
        return SettingsCatalogue.find(id, mobile)?.id ?? SettingsCatalogue.fallback;
    }
}
