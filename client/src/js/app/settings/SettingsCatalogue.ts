import { I18n } from "$lib/i18n";
import type { SettingsGroup, SettingsPane } from "./SettingsPane";

/**
 * Which settings panes exist, and for whom.
 *
 * One source for the sidebar, the phone's section list, the router and the deep links.
 * The mobile build is a different set of panes, decided by platform and never by width.
 */
export class SettingsCatalogue {
    /** Where settings opens, and where a link to a pane this build lacks lands. */
    static readonly fallback = "account";

    /**
     * A getter, not a field: the titles are read out of the catalog on access, and an
     * array literal on the class body would capture them in English at import, before any
     * language pack has loaded.
     *
     * `id` is a key — it is in the router, the deep links and the store — and is never
     * translated. Only `title` and `group` are copy. The ungrouped panes keep a literal
     * empty string rather than a marked one, because an empty msgid is the catalog header.
     */
    static get all(): readonly SettingsPane[] {
        const bedrock = I18n.t("Minecraft Bedrock");

        return [
            { id: "account", title: I18n.t("Account"), group: "", wide: false, desktopOnly: false },
            { id: "audio", title: I18n.t("Audio settings"), group: "", wide: false, desktopOnly: false },
            { id: "players", title: I18n.t("Players"), group: "", wide: true, desktopOnly: false },
            { id: "recordings", title: I18n.t("Recordings"), group: "", wide: false, desktopOnly: true },
            { id: "library", title: I18n.t("Audio library"), group: "", wide: true, desktopOnly: false },
            { id: "keybinds", title: I18n.t("Keybinds"), group: "", wide: false, desktopOnly: true },
            { id: "ws", title: I18n.t("WebSocket server"), group: "", wide: false, desktopOnly: false },
            { id: "about", title: I18n.t("About"), group: "", wide: false, desktopOnly: false },
            {
                id: "connect",
                title: I18n.t("Voice Chat Connect"),
                group: bedrock,
                wide: true,
                desktopOnly: false,
            },
        ];
    }

    static for(mobile: boolean): readonly SettingsPane[] {
        return mobile ? this.all.filter((pane) => !pane.desktopOnly) : this.all;
    }

    /** Grouped, in `all` order, so a group cannot jump the list by being renamed. */
    static groups(mobile: boolean): readonly SettingsGroup[] {
        const order: string[] = [];
        const byName = new Map<string, SettingsPane[]>();
        for (const pane of this.for(mobile)) {
            const existing = byName.get(pane.group);
            if (existing) {
                existing.push(pane);
                continue;
            }
            order.push(pane.group);
            byName.set(pane.group, [pane]);
        }
        return order.map((name) => ({ name, panes: byName.get(name) ?? [] }));
    }

    static find(id: string, mobile: boolean): SettingsPane | null {
        return this.for(mobile).find((pane) => pane.id === id) ?? null;
    }
}
