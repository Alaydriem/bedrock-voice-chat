import { I18n } from "$lib/i18n";
import type { AdminActionOutcome } from "../../bindings/AdminActionOutcome";
import type { AdminUserRow } from "../../bindings/AdminUserRow";
import type { Permission } from "../../bindings/Permission";
import type { PermissionEntry } from "../../bindings/PermissionEntry";
import type { Game } from "../../bindings/Game";
import type { ViewerIdentity } from "../managers/ViewerIdentity";
import type { ManagedPlayerRow, ManagedPlayerStatus, RosterBlock } from "./ManagedPlayerRow";

/** The three states the permission editor writes. Default clears the override. */
export type PermissionState = "default" | "allow" | "deny";

/**
 * The server roster, as the admin pane shows it.
 *
 * Every derivation is static and takes its inputs, so the pane holds only the query, the
 * page and which row is open. That is what makes the status precedence and the refusal
 * copy testable without a DOM.
 */
export class ManagePlayersView {
    /** The server clamps at 100; a settings pane shows what a settings pane can show. */
    static readonly PAGE_SIZE = 8;

    /** The backend counts pages from zero. */
    static readonly FIRST_PAGE = 0;

    /** Every permission the editor offers, in the order it renders them. */
    static readonly EDITABLE: readonly Permission[] = ["admin", "audio_upload", "audio_delete"];

    /** A slot nothing fills. Nearly the ground, so it holds its place without reading as a state. */
    static readonly BLOCK_OFF = "var(--color-rad-mortar-dead)";

    /**
     * One colour per permission, from the palette rather than a literal, so the strip
     * follows the theme it is painted on.
     */
    static readonly PERMISSION_COLOR: Readonly<Record<Permission, string>> = {
        admin: "var(--color-rad-brand-lift)",
        audio_upload: "var(--color-rad-scope-ok)",
        audio_delete: "var(--color-rad-warn)",
    };

    static rows(items: readonly AdminUserRow[]): readonly ManagedPlayerRow[] {
        return items.map((item) => this.row(item));
    }

    static row(item: AdminUserRow): ManagedPlayerRow {
        return {
            key: `${item.game}:${item.gamertag}`,
            gamertag: item.gamertag,
            game: item.game,
            status: this.status(item),
            banned: item.banished,
            permissions: item.permissions,
            added: this.added(item.created_at),
        };
    }

    /**
     * Banned outranks online.
     *
     * Banning closes the live session, so a banned row that still reads as connected is a
     * stale registry entry. Showing it as online would make a ban that worked look like a
     * ban that failed.
     */
    static status(item: AdminUserRow): ManagedPlayerStatus {
        if (item.banished) return "banned";
        return item.connected ? "online" : "offline";
    }

    /** Seconds since the epoch, in the reader's locale. */
    static added(createdAt: number | bigint): string {
        const seconds = Number(createdAt);
        if (!Number.isFinite(seconds) || seconds <= 0) return "—";
        return new Date(seconds * 1000).toLocaleDateString();
    }

    static pageCount(total: number, pageSize: number): number {
        if (pageSize <= 0) return 1;
        return Math.max(1, Math.ceil(total / pageSize));
    }

    static clampPage(page: number, total: number, pageSize: number): number {
        return Math.min(Math.max(this.FIRST_PAGE, page), this.pageCount(total, pageSize) - 1);
    }

    /**
     * Which of the three states a permission is in for this player.
     *
     * Read from the override list, never from the effective set: a default allow and an
     * explicit allow are identical in the effective set, and collapsing them would make
     * Default unreachable in the editor.
     */
    static state(entries: readonly PermissionEntry[], permission: Permission): PermissionState {
        const entry = entries.find((candidate) => candidate.permission === permission);
        if (!entry) return "default";
        return entry.effect === "allow" ? "allow" : "deny";
    }

    /** The game, as a reader sees it. The wire form is lowercase; a column heading is not. */
    static gameLabel(game: Game): string {
        return game.charAt(0).toUpperCase() + game.slice(1);
    }

    /**
     * The row's status strip: presence, then one slot per permission.
     *
     * Positional on purpose. Every row carries the same slots in the same order, so a
     * column of rows can be read downward; omitting an unheld permission would shift every
     * slot after it and make two rows incomparable.
     */
    static blocks(row: ManagedPlayerRow): readonly RosterBlock[] {
        const presence: RosterBlock = {
            color: this.presenceColor(row.status),
            label: this.statusLabel(row.status),
            on: true,
        };

        return [
            presence,
            ...this.EDITABLE.map((permission) => {
                const held = row.permissions.includes(permission);
                return {
                    color: held ? this.PERMISSION_COLOR[permission] : this.BLOCK_OFF,
                    label: this.label(permission),
                    on: held,
                };
            }),
        ];
    }

    /** Green for present, grey for away, and the fault colour for shut out. */
    static presenceColor(status: ManagedPlayerStatus): string {
        switch (status) {
            case "banned":
                return "var(--color-rad-fault)";
            case "online":
                return "var(--color-rad-ok)";
            default:
                return "var(--color-rad-line-2)";
        }
    }

    static statusLabel(status: ManagedPlayerStatus): string {
        switch (status) {
            case "banned":
                return I18n.t("Banned");
            case "online":
                return I18n.t("Online");
            default:
                return I18n.t("Offline");
        }
    }

    /** What the strip says out loud: presence, then whatever is actually held. */
    static blocksLabel(row: ManagedPlayerRow): string {
        return this.blocks(row)
            .filter((block) => block.on)
            .map((block) => block.label)
            .join(" · ");
    }

    /**
     * Whether this row is the signed-in operator.
     *
     * Compared on game and gamertag together, because two players in different games can
     * share a gamertag and are not the same person. An unknown identity matches nobody:
     * before introspect answers, no row may be assumed to be self.
     */
    static isSelf(row: ManagedPlayerRow, identity: ViewerIdentity | null): boolean {
        if (!identity) return false;
        return row.gamertag === identity.gamertag && row.game === identity.game;
    }

    static label(permission: Permission): string {
        switch (permission) {
            case "admin":
                return I18n.t("Administrator");
            case "audio_upload":
                return I18n.t("Upload sounds");
            case "audio_delete":
                return I18n.t("Delete sounds");
            default:
                return permission;
        }
    }

    static banFailure(outcome: AdminActionOutcome): string {
        switch (outcome) {
            case "conflict":
                return I18n.t("You cannot ban yourself.");
            case "forbidden":
                return I18n.t("You no longer hold the admin permission.");
            case "not_found":
                return I18n.t("That player is no longer on this server.");
            default:
                return I18n.t("The server refused that change.");
        }
    }

    static addFailure(outcome: AdminActionOutcome): string {
        switch (outcome) {
            case "conflict":
                return I18n.t("That player is already on the whitelist.");
            case "forbidden":
                return I18n.t("You no longer hold the admin permission.");
            case "invalid":
                return I18n.t("That is not a usable gamertag.");
            default:
                return I18n.t("The server refused that change.");
        }
    }

    static permissionFailure(outcome: AdminActionOutcome): string {
        switch (outcome) {
            case "conflict":
                return I18n.t("You cannot remove your own admin permission.");
            case "forbidden":
                return I18n.t("You no longer hold the admin permission.");
            case "invalid":
                return I18n.t("That permission does not exist on this server.");
            default:
                return I18n.t("The server refused that change.");
        }
    }
}
