import { I18n } from "#lib/i18n/index.js";
import type { ServerCheckFailure } from "../../bindings/ServerCheckFailure";

/**
 * What the sign-in screen says when a server check fails.
 *
 * The classification is done in Rust, from the error itself rather than its text, because the
 * text of an OS error arrives in the player's language.
 */
export class ServerCheckMessage {
    static for(failure: ServerCheckFailure): string {
        switch (failure.kind) {
            case "NameNotFound":
                return I18n.t("No server has that name. Check the spelling");
            case "Refused":
                return I18n.t("Server unreachable: nothing is accepting connections at that address");
            case "TimedOut":
                return I18n.t("Server unreachable: it did not answer in time");
            case "ConnectionLost":
                return I18n.t("Server unreachable: the connection was closed");
            case "WrongCertificateName":
                if (failure.presented.length === 0) {
                    return I18n.t("Server unreachable: the secure connection was refused");
                }
                return I18n.tf(
                    "That address belongs to {name}. It may be your game server or panel, not the voice server",
                    { name: failure.presented[0] },
                );
            case "TlsRejected":
                return I18n.t("Server unreachable: the secure connection was refused");
            case "NotBvc":
                return I18n.t("That address answered, but it is not a BVC server");
            case "ServerError":
                return I18n.tf(
                    "Server unreachable: it answered with an error (HTTP {status}). Try again later",
                    { status: failure.status },
                );
            case "UnreadableConfig":
                return I18n.t(
                    "That server's configuration cannot be read by this version. Check for an app update",
                );
            case "Other":
                return I18n.t("Server unreachable");
        }
    }
}
