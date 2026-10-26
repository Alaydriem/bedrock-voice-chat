import { describe, expect, it } from "vitest";
import { ServerAddressFormat } from "../../../js/app/login/ServerAddressFormat";

function errorOf(raw: string): string | null {
    const verdict = ServerAddressFormat.check(raw);
    return verdict.valid ? null : verdict.error;
}

function hostOf(raw: string): string | null {
    const verdict = ServerAddressFormat.check(raw);
    return verdict.valid ? verdict.host : null;
}

const SPACES = "Server addresses cannot contain spaces";
const EMAIL =
    "That looks like an email address. Enter the voice server's address, like voice.example.com";
const PATH = "Enter only the server address, without a path";
const NUMBER = "That is a number, not a server address";
const IP = "Enter the server's domain name, not an IP address";
const PORT_RANGE = "Port must be between 1 and 65535";
const GAME_PORT =
    "19132 is the Minecraft game port. Ask the server owner for the voice server address";
const FULL = "Enter the full server address, like voice.example.com";
const CHARACTERS = "Server addresses can only use letters, numbers, dots and hyphens";

/**
 * Every rejected input here was typed into the sign-in screen by a real player and reached
 * a request, per Sentry. Each one failed later with an error that did not say what was wrong
 * with what they typed.
 */
describe("ServerAddressFormat.check rejects", () => {
    it("an empty value", () => {
        expect(errorOf("")).toBe("Please enter a server URL");
        expect(errorOf("   ")).toBe("Please enter a server URL");
    });

    it("a value with spaces", () => {
        expect(errorOf("my world")).toBe(SPACES);
        expect(errorOf("AL TL9323")).toBe(SPACES);
    });

    it("spaces before an email shape", () => {
        expect(errorOf("a b@c")).toBe(SPACES);
    });

    it("an email address", () => {
        expect(errorOf("ethangood75@outlook.com")).toBe(EMAIL);
    });

    it("a path or a query", () => {
        expect(errorOf("bvc.example.com/api")).toBe(PATH);
        expect(errorOf("bvc.example.com?x=1")).toBe(PATH);
        expect(errorOf("bvc.example.com#top")).toBe(PATH);
    });

    it("a bare number", () => {
        expect(errorOf("19132")).toBe(NUMBER);
        expect(errorOf("04012011")).toBe(NUMBER);
    });

    it("an IP address of any range", () => {
        expect(errorOf("192.168.0.108")).toBe(IP);
        expect(errorOf("2.26.161.57")).toBe(IP);
        expect(errorOf("127.0.0.1:19132")).toBe(IP);
        expect(errorOf("[::1]:8443")).toBe(IP);
        expect(errorOf("https://[2001:db8::1]/")).toBe(IP);
    });

    it("a port out of range", () => {
        expect(errorOf("bvc.example.com:0")).toBe(PORT_RANGE);
        expect(errorOf("bvc.example.com:70000")).toBe(PORT_RANGE);
        expect(errorOf("bvc.example.com:")).toBe(PORT_RANGE);
        expect(errorOf("bvc.example.com:abc")).toBe(PORT_RANGE);
    });

    it("the Minecraft game port", () => {
        expect(errorOf("zetrex.net:19132")).toBe(GAME_PORT);
        expect(errorOf("zetrex.net:19133")).toBe(GAME_PORT);
    });

    it("localhost and single-label names", () => {
        expect(errorOf("localhost")).toBe(FULL);
        expect(errorOf("localhost:8443")).toBe(FULL);
        expect(errorOf("BedrockVoice")).toBe(FULL);
        expect(errorOf("realm")).toBe(FULL);
    });

    it("characters a hostname cannot hold", () => {
        expect(errorOf("Be15iwKa3mc=.com")).toBe(CHARACTERS);
        expect(errorOf("RB_OF_SEAL.aternos.me")).toBe(CHARACTERS);
        expect(errorOf("-bad.example.com")).toBe(CHARACTERS);
        expect(errorOf("bad-.example.com")).toBe(CHARACTERS);
        expect(errorOf("bvc..example.com")).toBe(CHARACTERS);
        expect(errorOf("bvc.example.com.")).toBe(CHARACTERS);
    });

    it("a top-level domain that cannot exist", () => {
        expect(errorOf("bvc.e")).toBe(FULL);
        expect(errorOf("bvc.123")).toBe(FULL);
    });
});

describe("ServerAddressFormat.check accepts", () => {
    it("a plain domain", () => {
        expect(hostOf("bvc.io")).toBe("bvc.io");
    });

    it("a domain with a non-standard port", () => {
        expect(hostOf("alaydriem.bedrockvc.stream:28280")).toBe("alaydriem.bedrockvc.stream:28280");
    });

    it("mixed case, lower-cased", () => {
        expect(hostOf("https://Bvc.Example.COM/")).toBe("bvc.example.com");
    });

    it("surrounding whitespace", () => {
        expect(hostOf("  bvc.example.com  ")).toBe("bvc.example.com");
    });

    it("an http scheme", () => {
        expect(hostOf("http://bvc.example.com")).toBe("bvc.example.com");
    });

    it("a punycode top-level domain", () => {
        expect(hostOf("voz.xn--p1ai")).toBe("voz.xn--p1ai");
    });
});
