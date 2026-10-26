import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { CredentialRejection } from "../../../js/app/auth/CredentialRejection";

/**
 * A background lookup that learns the server no longer accepts this device's certificate has
 * already signed the player out in Rust. The dashboard it ran on is then showing a session that
 * no longer exists, so it has to leave for the screen that says why.
 */
describe("CredentialRejection.handle", () => {
    let replaced: string[];

    beforeEach(() => {
        replaced = [];
        vi.spyOn(window.location, "replace").mockImplementation((url: string | URL) => {
            replaced.push(String(url));
        });
    });

    afterEach(() => vi.restoreAllMocks());

    it("leaves for CERT01 on a rejected certificate", () => {
        expect(CredentialRejection.handle("CERT_INVALID")).toBe(true);
        expect(replaced).toEqual(["/error?code=CERT01"]);
    });

    it("recognises the rejection inside an Error", () => {
        expect(CredentialRejection.handle(new Error("CERT_INVALID"))).toBe(true);
    });

    it("ignores any other failure", () => {
        expect(CredentialRejection.handle("Network error occurred")).toBe(false);
        expect(replaced).toEqual([]);
    });
});
