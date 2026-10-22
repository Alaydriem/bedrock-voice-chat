import { I18n } from "#lib/i18n/index.js";
import type { PlateAction } from "./PlateAction";
import type { ServerRosterEntry } from "./ServerRosterEntry";
import type { TallyItem } from "./TallyItem";

export interface PlateViewState {
    /** State in one word. `muted` is a check still running, not a good result. */
    readonly severity: "ok" | "warn" | "bad" | "muted";
    readonly chip: string;
    /** The primary button's label. */
    readonly action: string;
    readonly kind: PlateAction;
}

/**
 * What one plate says, from its status alone.
 *
 * The chip carries the whole verdict rather than a per-check summary, because the preflight
 * strip beside it already shows where a failure happened. This says what to do about it.
 */
export class PlateView {
    static of(entry: ServerRosterEntry): PlateViewState {
        switch (entry.status) {
            case "connect":
                return entry.slow
                    ? {
                          chip: I18n.t("Ready · slow"),
                          severity: "warn",
                          action: I18n.t("Connect"),
                          kind: "connect",
                      }
                    : {
                          chip: I18n.t("Ready"),
                          severity: "ok",
                          action: I18n.t("Connect"),
                          kind: "connect",
                      };

            case "reauth":
                return {
                    chip: I18n.t("Sign in needed"),
                    severity: "bad",
                    action: I18n.t("Sign in"),
                    kind: "signin",
                };

            /**
             * Both directions are blocked, and only one of them an update can fix. The
             * design's table treats them as one chip; pointing "Update needed" at somebody
             * whose client is the newer of the two sends them to look for an update that
             * does not exist and would not help.
             */
            case "version_mismatch":
                return entry.clientTooOld
                    ? {
                          chip: I18n.t("Update needed"),
                          severity: "warn",
                          action: I18n.t("Update"),
                          kind: "update",
                      }
                    : {
                          chip: I18n.t("Server is older"),
                          severity: "warn",
                          action: I18n.t("Blocked"),
                          kind: "blocked",
                      };

            /**
             * Voice arrives, over the slower of the two transports. A warning rather than a
             * clean result and rather than a blocker: connecting works, and saying nothing
             * about the path would leave somebody wondering why this server sounds worse
             * than the one beside it.
             */
            case "ws_fallback":
                return {
                    chip: I18n.t("Ready · fallback path"),
                    severity: "warn",
                    action: I18n.t("Connect"),
                    kind: "connect",
                };

            // Connecting would fail, so rechecking is the only thing worth offering.
            case "udp_blocked":
                return {
                    chip: I18n.t("Voice blocked"),
                    severity: "bad",
                    action: I18n.t("Recheck"),
                    kind: "recheck",
                };

            case "unreachable":
                return {
                    chip: I18n.t("Not answering"),
                    severity: "bad",
                    action: I18n.t("Recheck"),
                    kind: "recheck",
                };

            case "checking":
                return {
                    chip: I18n.t("Checking…"),
                    severity: "muted",
                    action: I18n.t("Connect"),
                    kind: "blocked",
                };
        }
    }

    /** Whether choosing this plate leads to the dashboard rather than anywhere else. */
    static isJoinable(entry: ServerRosterEntry): boolean {
        return entry.status === "connect" || entry.status === "ws_fallback";
    }

    /**
     * The bar under the list: how many servers are in each state, in one line.
     *
     * Counted rather than listed, because the plates above already name them. This is for
     * the glance that tells you whether the screen is worth reading closely.
     */
    static tally(entries: readonly ServerRosterEntry[]): readonly TallyItem[] {
        const labels = entries.map((entry) => PlateView.tallyLabel(entry));
        const seen = labels.filter((label, index) => labels.indexOf(label) === index);
        return seen.map((label) => {
            const first = entries[labels.indexOf(label)];
            return {
                label,
                count: labels.filter((other) => other === label).length,
                severity: PlateView.tallySeverity(first),
            };
        });
    }

    private static tallyLabel(entry: ServerRosterEntry): string {
        switch (entry.status) {
            case "connect":
                return entry.slow ? I18n.t("slow") : I18n.t("ready");
            case "reauth":
                return I18n.t("need sign-in");
            case "version_mismatch":
                return I18n.t("outdated");
            case "ws_fallback":
                return I18n.t("fallback path");
            case "udp_blocked":
                return I18n.t("voice blocked");
            case "unreachable":
                return I18n.t("not answering");
            case "checking":
                return I18n.t("checking");
        }
    }

    private static tallySeverity(entry: ServerRosterEntry): TallyItem["severity"] {
        switch (entry.status) {
            case "connect":
                return entry.slow ? "warn" : "ok";
            case "version_mismatch":
            case "ws_fallback":
                return "warn";
            case "reauth":
            case "udp_blocked":
            case "unreachable":
                return "bad";
            case "checking":
                return "busy";
        }
    }
}
