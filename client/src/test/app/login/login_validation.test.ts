import { beforeEach, describe, expect, it } from "vitest";
import { mockInvoke } from "../../tauri";
import Login from "../../../js/app/login";

/**
 * Submit and the inline readout must reject the same values with the same words, so the
 * sign-in validator delegates to the format check rather than keeping a looser one.
 */
describe("Login.validateServerUrl", () => {
    beforeEach(() => mockInvoke({}));

    it("rejects an email address that URL parsing accepts as user info", () => {
        expect(new Login().validateServerUrl("ethangood75@outlook.com")).toEqual({
            valid: false,
            error: "That looks like an email address. Enter the voice server's address, like voice.example.com",
        });
    });

    it("rejects a bare number that URL parsing turns into an IPv4 address", () => {
        expect(new Login().validateServerUrl("19132").valid).toBe(false);
    });

    it("accepts a server address", () => {
        expect(new Login().validateServerUrl("bvc.example.com")).toEqual({ valid: true });
    });

    // A capitalised scheme passes validation, so it has to reach the same host it was validated as.
    it("sanitizes an upper-case scheme and host to the address that was validated", () => {
        const login = new Login();
        expect(login.validateServerUrl("HTTPS://Bvc.Example.com").valid).toBe(true);
        expect(login.sanitizeServerUrl("HTTPS://Bvc.Example.com/")).toBe("https://bvc.example.com");
        expect(login.sanitizeServerUrl("Http://bvc.example.com:8443")).toBe(
            "https://bvc.example.com:8443",
        );
    });

    it("gives a code sign-in no target until its host is a valid address", () => {
        const login = new Login();
        expect(login.codeLoginTarget("code@realm")).toBeNull();
        expect(login.codeLoginTarget("code@bvc.example.com")).toBe("https://bvc.example.com");
    });
});
