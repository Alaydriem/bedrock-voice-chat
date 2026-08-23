import { I18n } from "$lib/i18n";
import type { PlayerSettingsRow } from "../../bindings/PlayerSettingsRow";
import GameNameUtils from "../utils/GameNameUtils";
import type { PlayerRow } from "./PlayerRow";

/** Which players the pane is showing. */
export type PlayerScope = "adjusted" | "all";

/**
 * Everyone who has been within earshot, and what you decided about them.
 *
 * The list is written by proximity, not by choice: a player entering earshot is stamped at
 * 100% and unmuted whether or not you ever touch them. So the raw list is hundreds of rows
 * carrying no decision, and the segment defaults to the ones that do. Search serves the other
 * case — a name you remember and want to turn down before they are back.
 *
 * Every derivation here is static and takes its inputs, so the pane holds only the scope, the
 * query and the page. That is what makes the sort and the empty states testable without a DOM.
 */
export class PlayersView {
    static readonly PER_PAGE = 6;

    /**
     * A row carries a decision when it is muted or no longer at unity gain.
     *
     * Exact, and it must stay exact: this is the same rule as
     * `PlayerGainSettings::is_adjusted` in Rust, which decides what the pruner may delete. A
     * tolerance here and not there means a gain of 0.9995 is kept on disk forever but hidden
     * from the Adjusted list — the user cannot see or undo a setting that is still applying.
     * A tolerance in *both* would be worse: it would make a volume somebody deliberately set a
     * hair off unity prunable after thirty days.
     *
     * Exactness is safe because gain only ever arrives from a `step="0.05"` slider or from a
     * clamped `f32` on the control plane, never from arithmetic that could drift.
     *
     * `lastSeen` is deliberately not part of this — proximity stamps everybody, so counting a
     * stamp would make Adjusted mean the whole server.
     */
    static isAdjusted(row: { gain: number; muted: boolean }): boolean {
        return row.muted || row.gain !== 1;
    }

    static ago(lastSeen: number | null, now: number = Date.now()): string {
        if (lastSeen === null) return "not seen since this was added";
        const minutes = Math.max(0, Math.round((now - lastSeen) / 60_000));
        if (minutes < 1) return "just now";
        if (minutes < 60) return `${minutes} min ago`;
        const hours = Math.round(minutes / 60);
        return hours < 24 ? `${hours} h ago` : `${Math.round(hours / 24)} d ago`;
    }

    static row(source: PlayerSettingsRow, now: number = Date.now()): PlayerRow {
        const gain = source.settings.gain;
        const muted = source.settings.muted;
        const lastSeen = source.settings.last_seen ?? null;

        return {
            cn: source.key.cn,
            name: GameNameUtils.stripPrefix(source.key.cn),
            gain,
            muted,
            lastSeen,
            adjusted: this.isAdjusted({ gain, muted }),
            seen: this.ago(lastSeen, now),
            readout: muted ? "muted" : `${Math.round(gain * 100)}%`,
        };
    }

    static rows(sources: readonly PlayerSettingsRow[], now: number = Date.now()): PlayerRow[] {
        return sources.map((source) => this.row(source, now));
    }

    /**
     * The rows the pane should show, filtered and ordered.
     *
     * Muted first: "why can't I hear them" is the question this pane answers, and the answer
     * should never be on page two. Recency breaks the tie, so the people you were just around
     * come before the ones you passed last week.
     */
    /**
     * `pinned` is a row the user is currently interacting with. It survives the scope filter
     * so a slider cannot delete itself out from under the pointer: dragging back to exactly
     * 1.00 makes a row stop being adjusted, and under the Adjusted segment that would unmount
     * the input mid-drag. It still obeys the search, which the user is not touching.
     */
    static matching(
        rows: readonly PlayerRow[],
        scope: PlayerScope,
        query: string,
        pinned: string | null = null,
    ): PlayerRow[] {
        const wanted = query.trim().toLowerCase();
        return rows
            .filter((row) => {
                if (scope === "adjusted" && !row.adjusted && row.cn !== pinned) return false;
                return !wanted || row.name.toLowerCase().includes(wanted);
            })
            .sort((a, b) => {
                if (a.muted !== b.muted) return a.muted ? -1 : 1;
                return (b.lastSeen ?? 0) - (a.lastSeen ?? 0);
            });
    }

    static pageCount(matching: readonly PlayerRow[]): number {
        return Math.max(1, Math.ceil(matching.length / this.PER_PAGE));
    }

    /** How many numbered buttons the pager will ever draw, ellipses and edges included. */
    static readonly PAGER_SLOTS = 7;

    /**
     * The page numbers to draw, with `null` for a gap.
     *
     * Six rows a page against this pane's own premise — "anyone who comes within earshot", i.e.
     * hundreds of rows on a busy server — is fifty-plus buttons wrapping across the card. A
     * window keeps it to a fixed width: always the first and last page, always the neighbours
     * of the current one, and a gap where the run is broken.
     */
    static pageWindow(page: number, pages: number): (number | null)[] {
        if (pages <= this.PAGER_SLOTS) {
            return Array.from({ length: pages }, (_, index) => index);
        }

        // One slot each for the first page, the last page and the two gaps leaves three for the
        // run around the current page.
        const span = 1;
        const first = 0;
        const last = pages - 1;
        const from = Math.max(first + 1, Math.min(page - span, last - 2 * span - 1));
        const to = Math.min(last - 1, Math.max(page + span, first + 2 * span + 1));

        const window: (number | null)[] = [first];
        if (from > first + 1) window.push(null);
        for (let index = from; index <= to; index += 1) window.push(index);
        if (to < last - 1) window.push(null);
        window.push(last);
        return window;
    }

    /** One page of rows, clamping a page index that a filter change left out of range. */
    static page(matching: readonly PlayerRow[], page: number): PlayerRow[] {
        const clamped = Math.min(Math.max(0, page), this.pageCount(matching) - 1);
        return matching.slice(clamped * this.PER_PAGE, clamped * this.PER_PAGE + this.PER_PAGE);
    }

    /** The chip above the list: what is being counted, and how much of it there is. */
    static meta(rows: readonly PlayerRow[], scope: PlayerScope): string {
        const adjusted = rows.filter((row) => row.adjusted).length;
        if (scope === "adjusted") return `${adjusted} adjusted`;
        return `${rows.length} player${rows.length === 1 ? "" : "s"} · ${adjusted} adjusted`;
    }

    /**
     * Three different absences, three different sentences.
     *
     * "No results" for a list you have never added to is the least useful of the three: it
     * reads as a failure when nothing has gone wrong yet.
     */
    static empty(scope: PlayerScope, query: string): { title: string; note: string } {
        if (query.trim()) {
            return {
                title: I18n.t("Nobody matches that"),
                note:
                    scope === "adjusted"
                        ? I18n.t("Nobody you have changed has that name.")
                        : I18n.t("Nobody here has that name."),
            };
        }
        if (scope === "adjusted") {
            return {
                title: I18n.t("Nothing changed yet"),
                note: I18n.t("Change someone's volume on the dashboard and they show up here."),
            };
        }
        return {
            title: I18n.t("Nobody yet"),
            note: I18n.t("People appear here once you have been near them on this server."),
        };
    }
}
