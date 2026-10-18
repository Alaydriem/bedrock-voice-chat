import { I18n } from "#lib/i18n/index.js";
import type { ServerRosterEntry } from "../ServerRosterEntry";
import type { PreflightStep } from "./PreflightStep";

export type VerdictSeverity = "ok" | "warn" | "bad";

export interface Verdict {
    readonly severity: VerdictSeverity;
    readonly sentence: string;
}

/**
 * The preflight in one sentence, first failing check wins.
 *
 * Somebody reading this has a problem. A summary that averages four results into "mostly
 * fine" leaves them to find the one line that is not, and a wall of green with one red
 * line in the middle makes them do the diagnosis themselves.
 */
export class PreflightVerdict {
    static of(entry: ServerRosterEntry): Verdict {
        if (entry.status === "checking") {
            return { severity: "ok", sentence: I18n.t("Checking…") };
        }

        const failed = entry.steps.find((step) => step.state === "bad");
        if (failed) return PreflightVerdict.forFailure(failed, entry);

        // Named before the round-trip warning, which is the other thing that turns a row
        // amber. Both say "voice works, but slower"; only one of them has a cause worth
        // stating, and it is not one the player can do anything about by waiting.
        if (entry.status === "ws_fallback") {
            return {
                severity: "warn",
                sentence: I18n.tf(
                    "UDP {port} to this server is blocked, so voice will use the slower fallback path over TCP. It works — expect more delay when the network is busy.",
                    { port: entry.quicPort },
                ),
            };
        }

        if (entry.steps.some((step) => step.state === "warn")) {
            return {
                severity: "warn",
                sentence: I18n.tf(
                    "About {ms} ms for your voice to reach this server. Voice works, with delay you will notice.",
                    { ms: Math.round(entry.rtt / 2) },
                ),
            };
        }

        return { severity: "ok", sentence: I18n.t("Everything looks fine.") };
    }

    private static forFailure(failed: PreflightStep, entry: ServerRosterEntry): Verdict {
        switch (failed.name) {
            case "credentials":
                return {
                    severity: "bad",
                    sentence: I18n.t(
                        "You are not signed in to this server any more. Sign in again to connect.",
                    ),
                };

            case "handshake":
                return {
                    severity: "bad",
                    sentence:
                        entry.status === "reauth"
                            ? I18n.t(
                                  "This server refused your saved sign-in. Sign in again to connect.",
                              )
                            : I18n.t(
                                  "This server is not answering. Nothing is wrong on this device — ask whoever runs it.",
                              ),
                };

            case "protocol": {
                const versions = { server: entry.serverVersion, client: entry.clientVersion };
                return {
                    severity: "bad",
                    sentence: entry.clientTooOld
                        ? I18n.tf(
                              "This server speaks protocol {server} and your client speaks {client}. Update the client to connect.",
                              versions,
                          )
                        : I18n.tf(
                              "This server speaks protocol {server} and your client speaks {client}. Whoever runs it has to update it.",
                              versions,
                          ),
                };
            }

            case "voicePath":
                return {
                    severity: "bad",
                    sentence: I18n.tf(
                        "UDP {port} to this server is blocked and it offers no fallback path, so voice cannot connect at all. Check the network or firewall you are behind, then recheck.",
                        { port: entry.quicPort },
                    ),
                };
        }
    }
}
