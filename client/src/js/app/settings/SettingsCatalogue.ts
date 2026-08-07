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

    static readonly all: readonly SettingsPane[] = [
        { id: "account", title: "Account", group: "", wide: false, desktopOnly: false },
        { id: "audio", title: "Audio settings", group: "", wide: false, desktopOnly: false },
        { id: "players", title: "Players", group: "", wide: true, desktopOnly: false },
        { id: "recordings", title: "Recordings", group: "", wide: false, desktopOnly: true },
        { id: "library", title: "Audio library", group: "", wide: true, desktopOnly: false },
        { id: "keybinds", title: "Keybinds", group: "", wide: false, desktopOnly: true },
        { id: "ws", title: "WebSocket server", group: "", wide: false, desktopOnly: false },
        { id: "about", title: "About", group: "", wide: false, desktopOnly: false },
        {
            id: "proxy",
            title: "Proxy Connect",
            group: "Minecraft Bedrock",
            wide: true,
            desktopOnly: false,
        },
        {
            id: "realms",
            title: "Realms Connect",
            group: "Minecraft Bedrock",
            wide: true,
            desktopOnly: false,
        },
    ];

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
