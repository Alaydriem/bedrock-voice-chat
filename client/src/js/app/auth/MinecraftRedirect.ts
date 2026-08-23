/**
 * The redirect the mobile Microsoft sign-in comes back through.
 *
 * One constant, because OAuth compares two of them. The authorization request states a
 * redirect and the token exchange has to present the same string back, or the provider
 * rejects the grant — and it reports that as a bad code, not as a bad redirect, which
 * sends anyone reading the log after the code instead of after the mismatch.
 *
 * It was written out by hand in both places: `Login` for the authorize URL and
 * `AuthCallbackHandler` for the exchange. Two literals that must agree and nothing to make
 * them.
 *
 * Desktop does not come through here. It signs in through an in-app window against
 * `oauth20_desktop.srf`, whose value is a Rust constant used for both halves.
 */
export default class MinecraftRedirect {
  static readonly URI = "bedrock-voice-chat://auth";

  /**
   * Escaped for use as a query-parameter value.
   *
   * The authorize URL was interpolating the raw value, so a `:` and two `/` went into a
   * query string unescaped and the provider was left to interpret them.
   */
  static encoded(): string {
    return encodeURIComponent(MinecraftRedirect.URI);
  }
}
