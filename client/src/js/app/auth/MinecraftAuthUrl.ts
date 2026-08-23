import MinecraftRedirect from "./MinecraftRedirect";

/**
 * The Microsoft authorization URL for the mobile sign-in.
 *
 * Its own class so the URL can be asserted without a server to fetch a client id from.
 * Built inline inside a network call, the one property that matters — that its redirect is
 * the same string the token exchange presents — had nowhere to be checked.
 */
export default class MinecraftAuthUrl {
  static readonly ENDPOINT = "https://login.live.com/oauth20_authorize.srf";

  /** Xbox Live sign-in, plus a refresh token. */
  static readonly SCOPE = "XboxLive.signin offline_access";

  static build(clientId: string, state: string): string {
    const params = [
      `client_id=${encodeURIComponent(clientId)}`,
      "response_type=code",
      `redirect_uri=${MinecraftRedirect.encoded()}`,
      `scope=${encodeURIComponent(MinecraftAuthUrl.SCOPE)}`,
      `state=${encodeURIComponent(state)}`,
      // Two accounts signed into one browser otherwise resolve to whichever the browser
      // considers current, with nothing on screen to say which one signed in.
      "prompt=select_account",
    ];
    return `${MinecraftAuthUrl.ENDPOINT}?${params.join("&")}`;
  }
}
