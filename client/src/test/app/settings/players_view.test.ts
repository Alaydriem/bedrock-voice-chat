import { describe, expect, it } from "vitest";
import { PlayersView } from "../../../js/app/settings/PlayersView";
import type { PlayerSettingsRow } from "../../../js/bindings/PlayerSettingsRow";

const NOW = 1_753_732_440_000;

function source(
    cn: string,
    gain: number,
    muted: boolean,
    lastSeen: number | null = NOW,
): PlayerSettingsRow {
    return {
        key: { server: "bvc.example.com", cn },
        settings: { gain, muted, last_seen: lastSeen },
    } as PlayerSettingsRow;
}

describe("PlayersView.isAdjusted", () => {
    it("does not count a player left exactly where proximity put them", () => {
        expect(PlayersView.isAdjusted({ gain: 1.0, muted: false })).toBe(false);
    });

    // Exact, matching `PlayerGainSettings::is_adjusted` in Rust — which is what the pruner
    // consults. A tolerance on only one side keeps a row on disk forever while hiding it from
    // the list that would let the user undo it.
    it("counts any departure from unity, however small", () => {
        expect(PlayersView.isAdjusted({ gain: 0.999, muted: false })).toBe(true);
        expect(PlayersView.isAdjusted({ gain: 1 + Number.EPSILON, muted: false })).toBe(true);
    });

    // Every value the slider can produce, so the two implementations are checked to agree
    // across the whole real input domain rather than at a couple of hand-picked points.
    it("agrees with the Rust rule at every step the slider can produce", () => {
        for (let step = 0; step <= 30; step += 1) {
            const gain = step * 0.05;
            expect(PlayersView.isAdjusted({ gain, muted: false })).toBe(gain !== 1);
        }
    });

    it("counts a real change and a mute", () => {
        expect(PlayersView.isAdjusted({ gain: 0.5, muted: false })).toBe(true);
        expect(PlayersView.isAdjusted({ gain: 1.0, muted: true })).toBe(true);
    });
});

describe("PlayersView.ago", () => {
    it("says so plainly when the row was never stamped", () => {
        expect(PlayersView.ago(null, NOW)).toBe("not seen since this was added");
    });

    it("crosses the minute, hour and day boundaries", () => {
        // Rounded, not floored, so "just now" runs to the half-minute rather than the minute.
        expect(PlayersView.ago(NOW - 20_000, NOW)).toBe("just now");
        expect(PlayersView.ago(NOW - 40_000, NOW)).toBe("1 min ago");
        expect(PlayersView.ago(NOW - 5 * 60_000, NOW)).toBe("5 min ago");
        expect(PlayersView.ago(NOW - 90 * 60_000, NOW)).toBe("2 h ago");
        expect(PlayersView.ago(NOW - 50 * 3_600_000, NOW)).toBe("2 d ago");
    });

    // A clock that jumped backwards, or a stamp written a moment in the future, must not
    // render as a negative age.
    it("never reports a negative age", () => {
        expect(PlayersView.ago(NOW + 60_000, NOW)).toBe("just now");
    });
});

describe("PlayersView.matching", () => {
    // "Why can't I hear them" is the question this pane answers, and the answer should never
    // be on page two — not even behind somebody seen more recently.
    it("puts a muted player above a more recently seen unmuted one", () => {
        const rows = PlayersView.rows(
            [
                source("minecraft:Recent", 1.0, false, NOW),
                source("minecraft:Muted", 1.0, true, NOW - 3_600_000),
            ],
            NOW,
        );

        const ordered = PlayersView.matching(rows, "all", "");
        expect(ordered.map((row) => row.name)).toEqual(["Muted", "Recent"]);
    });

    it("orders unmuted players by recency", () => {
        const rows = PlayersView.rows(
            [
                source("minecraft:Older", 0.5, false, NOW - 7_200_000),
                source("minecraft:Newer", 0.5, false, NOW),
            ],
            NOW,
        );

        expect(PlayersView.matching(rows, "all", "").map((row) => row.name)).toEqual([
            "Newer",
            "Older",
        ]);
    });

    it("hides an untouched player under Adjusted and shows them under Everyone", () => {
        const rows = PlayersView.rows(
            [source("minecraft:Plain", 1.0, false), source("minecraft:Quiet", 0.4, false)],
            NOW,
        );

        expect(PlayersView.matching(rows, "adjusted", "").map((r) => r.name)).toEqual(["Quiet"]);
        expect(PlayersView.matching(rows, "all", "").map((r) => r.name).sort()).toEqual([
            "Plain",
            "Quiet",
        ]);
    });

    // A slider dragged back to exactly 100% stops being "adjusted" mid-drag. Without pinning
    // the row being held, the Adjusted segment filters it out, the keyed each-block destroys
    // the range input under the pointer, and the drag dies — leaving the user unable to put
    // somebody back to normal from the very list that exists to let them.
    it("keeps the row being dragged even once it is no longer adjusted", () => {
        const rows = PlayersView.rows([source("minecraft:Alaydriem", 1.0, false)], NOW);

        expect(PlayersView.matching(rows, "adjusted", "")).toHaveLength(0);
        expect(
            PlayersView.matching(rows, "adjusted", "", "minecraft:Alaydriem").map((r) => r.name),
        ).toEqual(["Alaydriem"]);
    });

    // Pinning survives the scope filter, not the search — the user is not typing mid-drag, and
    // a pinned row appearing under a query it does not match would be a ghost.
    it("does not let a pinned row escape the search", () => {
        const rows = PlayersView.rows([source("minecraft:Alaydriem", 1.0, false)], NOW);
        expect(PlayersView.matching(rows, "adjusted", "zzz", "minecraft:Alaydriem")).toHaveLength(
            0,
        );
    });

    // Typed in a hurry, in whatever case the person remembers.
    it("matches a name case-insensitively and ignores surrounding space", () => {
        const rows = PlayersView.rows([source("minecraft:Alaydriem", 0.5, false)], NOW);
        expect(PlayersView.matching(rows, "all", "  ALAY ").map((r) => r.name)).toEqual([
            "Alaydriem",
        ]);
    });

    // The row key is `minecraft:Alaydriem`, but nobody searches for the prefix — and if the
    // prefix were searchable, one character would match every player at once.
    it("searches the gamertag rather than the game prefix", () => {
        const rows = PlayersView.rows([source("minecraft:Alaydriem", 0.5, false)], NOW);
        expect(PlayersView.matching(rows, "all", "minecraft")).toHaveLength(0);
    });
});

describe("PlayersView paging", () => {
    const many = PlayersView.rows(
        Array.from({ length: 14 }, (_, i) => source(`minecraft:P${i}`, 0.5, false, NOW - i * 1000)),
        NOW,
    );

    it("fills a page and reports the count", () => {
        expect(PlayersView.pageCount(many)).toBe(3);
        expect(PlayersView.page(many, 0)).toHaveLength(PlayersView.PER_PAGE);
        expect(PlayersView.page(many, 2)).toHaveLength(2);
    });

    // Narrowing the filter while on a later page must not leave the list blank with no way
    // back — the page index is clamped rather than trusted.
    it("clamps a page index the filter left out of range", () => {
        expect(PlayersView.page(many, 99)).toEqual(PlayersView.page(many, 2));
        expect(PlayersView.page(many, -3)).toEqual(PlayersView.page(many, 0));
    });
});

describe("PlayersView.pageWindow", () => {
    it("draws every page when they all fit", () => {
        expect(PlayersView.pageWindow(0, 5)).toEqual([0, 1, 2, 3, 4]);
    });

    // Six rows a page against "anyone who comes within earshot" is fifty-plus buttons wrapping
    // across the card. The window is what keeps the pager a fixed width.
    it("never draws more slots than it promises, however many pages there are", () => {
        for (const page of [0, 3, 40, 98, 99]) {
            expect(PlayersView.pageWindow(page, 100).length).toBeLessThanOrEqual(
                PlayersView.PAGER_SLOTS,
            );
        }
    });

    it("always keeps the first page, the last page and the current one reachable", () => {
        for (const page of [0, 1, 50, 98, 99]) {
            const window = PlayersView.pageWindow(page, 100);
            expect(window).toContain(0);
            expect(window).toContain(99);
            expect(window).toContain(page);
        }
    });

    it("marks the break with a gap rather than running the numbers together", () => {
        const middle = PlayersView.pageWindow(50, 100);
        expect(middle[1]).toBeNull();
        expect(middle[middle.length - 2]).toBeNull();

        // At the first page the run already starts at the edge, so there is nothing to elide
        // on the left — only on the right.
        const start = PlayersView.pageWindow(0, 100);
        expect(start[1]).not.toBeNull();
        expect(start[start.length - 2]).toBeNull();

        const end = PlayersView.pageWindow(99, 100);
        expect(end[1]).toBeNull();
        expect(end[end.length - 2]).not.toBeNull();
    });
});

describe("PlayersView copy", () => {
    it("counts differently depending on which segment is showing", () => {
        const rows = PlayersView.rows(
            [source("minecraft:A", 1.0, false), source("minecraft:B", 0.4, false)],
            NOW,
        );

        expect(PlayersView.meta(rows, "adjusted")).toBe("1 adjusted");
        expect(PlayersView.meta(rows, "all")).toBe("2 players · 1 adjusted");
    });

    it("gives each of the three absences its own sentence", () => {
        const searched = PlayersView.empty("all", "zzz");
        const nothingAdjusted = PlayersView.empty("adjusted", "");
        const nobodyAtAll = PlayersView.empty("all", "");

        expect(searched.title).toBe("Nobody matches that");
        expect(nothingAdjusted.title).toBe("Nothing changed yet");
        expect(nobodyAtAll.title).toBe("Nobody yet");
        expect(new Set([searched.note, nothingAdjusted.note, nobodyAtAll.note]).size).toBe(3);
    });

    it("says which list came up empty when a search is running", () => {
        // "Nobody matches that" under Adjusted has to say it searched the adjusted list, or it
        // reads as the player being absent from the server rather than from this segment.
        expect(PlayersView.empty("adjusted", "zzz").note).toContain("you have changed");
        expect(PlayersView.empty("all", "zzz").note).not.toContain("you have changed");
    });
});

describe("PlayersView.row", () => {
    it("reads out a percentage, or muted when muted", () => {
        expect(PlayersView.row(source("minecraft:A", 1.45, false), NOW).readout).toBe("145%");
        expect(PlayersView.row(source("minecraft:A", 1.45, true), NOW).readout).toBe("muted");
    });

    // The command takes the canonical key; the human reads the bare name. Both are carried so
    // the pane never has to recompose either one.
    it("keeps the canonical key and the display name apart", () => {
        const row = PlayersView.row(source("minecraft:Alaydriem", 1.0, false), NOW);
        expect(row.cn).toBe("minecraft:Alaydriem");
        expect(row.name).toBe("Alaydriem");
    });
});
