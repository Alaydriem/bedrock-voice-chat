import { I18n } from "$lib/i18n";
import type { Permission } from "../../bindings/Permission";
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
            { id: "players", title: I18n.t("Player audio levels"), group: "", wide: true, desktopOnly: false },
            { id: "recordings", title: I18n.t("Recordings"), group: "", wide: false, desktopOnly: true },
            { id: "library", title: I18n.t("Audio library"), group: "", wide: true, desktopOnly: false },
            { id: "keybinds", title: I18n.t("Keybinds"), group: "", wide: false, desktopOnly: true },
            { id: "ws", title: I18n.t("WebSocket server"), group: "", wide: false, desktopOnly: false },
            { id: "about", title: I18n.t("About"), group: "", wide: false, desktopOnly: false },
            {
                id: "manage-players",
                title: I18n.t("Manage Players"),
                group: I18n.t("Server"),
                wide: true,
                desktopOnly: false,
                requires: "admin",
            },
            {
                id: "connect",
                title: I18n.t("Voice Chat Connect"),
                group: bedrock,
                wide: true,
                desktopOnly: false,
            },
        ];
    }

    /**
     * Two filters, and neither is a preference: a desktop-only pane does not exist in the
     * mobile build, and a pane behind a permission does not exist for a viewer without it.
     * Both apply to `find` as well as to the list, or a deep link walks past the gate.
     */
    static for(mobile: boolean, permissions: readonly Permission[] = []): readonly SettingsPane[] {
        return this.all.filter((pane) => {
            if (mobile && pane.desktopOnly) return false;
            if (pane.requires && !permissions.includes(pane.requires)) return false;
            return true;
        });
    }

    /** Grouped, in `all` order, so a group cannot jump the list by being renamed. */
    static groups(
        mobile: boolean,
        permissions: readonly Permission[] = [],
    ): readonly SettingsGroup[] {
        const order: string[] = [];
        const byName = new Map<string, SettingsPane[]>();
        for (const pane of this.for(mobile, permissions)) {
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

    static find(
        id: string,
        mobile: boolean,
        permissions: readonly Permission[] = [],
    ): SettingsPane | null {
        return this.for(mobile, permissions).find((pane) => pane.id === id) ?? null;
    }

    /**
     * The lookup a URL resolves through, which applies the platform filter and not the
     * permission gate.
     *
     * Resolving a path is synchronous and happens the moment the URL changes; the
     * permission set arrives later, from the keyring. Gating here would resolve every
     * permission-gated pane to the fallback and render the wrong pane under the right
     * URL — a sidebar item that navigates and then appears to do nothing. The gate is
     * `find`, applied by the sidebar and by the render, where the permissions are known.
     */
    static resolve(id: string, mobile: boolean): SettingsPane | null {
        return this.all.find((pane) => pane.id === id && !(mobile && pane.desktopOnly)) ?? null;
    }
}
