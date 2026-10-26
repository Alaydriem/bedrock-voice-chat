import { describe, expect, it } from "vitest";
import { ServerCheckMessage } from "../../../js/app/login/ServerCheckMessage";

const TLS_REJECTED = "Server unreachable: the secure connection was refused";

describe("ServerCheckMessage.for", () => {
    it("says a name does not exist", () => {
        expect(ServerCheckMessage.for({ kind: "NameNotFound" })).toBe(
            "No server has that name. Check the spelling",
        );
    });

    it("says nothing accepted the connection", () => {
        expect(ServerCheckMessage.for({ kind: "Refused" })).toBe(
            "Server unreachable: nothing is accepting connections at that address",
        );
    });

    it("says the server did not answer in time", () => {
        expect(ServerCheckMessage.for({ kind: "TimedOut" })).toBe(
            "Server unreachable: it did not answer in time",
        );
    });

    it("says the connection was closed", () => {
        expect(ServerCheckMessage.for({ kind: "ConnectionLost" })).toBe(
            "Server unreachable: the connection was closed",
        );
    });

    // A player who typed their game host or panel learns which host they reached.
    it("names the host the certificate belongs to", () => {
        const message = ServerCheckMessage.for({
            kind: "WrongCertificateName",
            presented: ["panel.restcraft.net"],
        });
        expect(message).toBe(
            "That address belongs to panel.restcraft.net. It may be your game server or panel, not the voice server",
        );
    });

    it("falls back to the TLS message when the certificate names nothing", () => {
        expect(ServerCheckMessage.for({ kind: "WrongCertificateName", presented: [] })).toBe(
            TLS_REJECTED,
        );
    });

    it("says the secure connection was refused", () => {
        expect(ServerCheckMessage.for({ kind: "TlsRejected" })).toBe(TLS_REJECTED);
    });

    it("says the address is not a BVC server", () => {
        expect(ServerCheckMessage.for({ kind: "NotBvc", status: 401 })).toBe(
            "That address answered, but it is not a BVC server",
        );
    });

    it("tells a player whose server is erroring to try again, naming the status", () => {
        expect(ServerCheckMessage.for({ kind: "ServerError", status: 503 })).toBe(
            "Server unreachable: it answered with an error (HTTP 503). Try again later",
        );
    });

    it("sends a player whose server speaks a newer configuration to an update", () => {
        expect(ServerCheckMessage.for({ kind: "UnreadableConfig" })).toBe(
            "That server's configuration cannot be read by this version. Check for an app update",
        );
    });

    it("falls back to a plain unreachable", () => {
        expect(ServerCheckMessage.for({ kind: "Other" })).toBe("Server unreachable");
    });
});
