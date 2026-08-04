import { describe, expect, it } from "vitest";
import MinecraftAuthUrl from "../../js/app/auth/MinecraftAuthUrl";
import MinecraftRedirect from "../../js/app/auth/MinecraftRedirect";

/**
 * OAuth compares the redirect twice: the authorization request states one, and the token
 * exchange has to present the same string. A mismatch is refused as a bad authorization
 * code — the provider does not say the redirect was the problem — so this is a failure that
 * reads as an expired or already-used code, and on this app's error page as an account being
 * refused by the server.
 *
 * These two values were written out by hand in separate files. Nothing failed at build time
 * when they diverged, and nothing failed at run time either until Microsoft answered.
 */
describe("the mobile authorization URL", () => {
  it("states the same redirect the token exchange presents", () => {
    const url = new URL(MinecraftAuthUrl.build("00000000402b5328", "a-state-token"));
    expect(url.searchParams.get("redirect_uri")).toBe(MinecraftRedirect.URI);
  });

  /**
   * A custom scheme carries `:` and `//`, which are delimiters in a query string. The URL
   * was built by interpolating the raw value, leaving the provider to interpret them.
   */
  it("escapes the redirect rather than interpolating a raw scheme", () => {
    const raw = MinecraftAuthUrl.build("client", "state");
    expect(raw).toContain(`redirect_uri=${encodeURIComponent(MinecraftRedirect.URI)}`);
    expect(raw).not.toContain("redirect_uri=bedrock-voice-chat://");
  });

  // The state is echoed back and compared to what was stored. A value that survives the
  // round trip only when it happens to contain no reserved characters is a latent failure.
  it("escapes the state token", () => {
    const url = new URL(MinecraftAuthUrl.build("client", "a token&with=reserved chars"));
    expect(url.searchParams.get("state")).toBe("a token&with=reserved chars");
  });

  it("asks which account, since a browser can hold several", () => {
    const url = new URL(MinecraftAuthUrl.build("client", "state"));
    expect(url.searchParams.get("prompt")).toBe("select_account");
  });

  it("requests Xbox Live sign-in with a refresh token", () => {
    const url = new URL(MinecraftAuthUrl.build("client", "state"));
    expect(url.searchParams.get("scope")).toBe("XboxLive.signin offline_access");
    expect(url.searchParams.get("response_type")).toBe("code");
  });
});
