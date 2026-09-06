/**
 * Everything about the site that is structural rather than editorial: accents,
 * feature gates, spectrum order, and the constants the inline scripts need.
 *
 * Link targets are NOT here. A label and its href are one fact, so they sit
 * together in src/copy/en.ts. The only URL-ish constants left are the two the
 * client-side scripts build requests from, and they are not rendered as hrefs.
 */

/**
 * Versions come from the build environment so a release does not require a
 * commit to the site. The fallbacks are what ships if the variables are unset.
 *
 * NOTE: release.yml currently has an `update-gh-pages-version` job that
 * rewrites version strings on the gh-pages branch directly. Once the site is
 * built rather than hand-committed, that job should set PUBLIC_BVC_VERSION and
 * trigger a rebuild instead of editing published output.
 */
export const VERSION = import.meta.env.PUBLIC_BVC_VERSION ?? '1.0.0-beta.20';
export const MODS_VERSION = import.meta.env.PUBLIC_BVC_MODS_VERSION ?? `mods-v${VERSION}`;

/* ------------------------------------------------------------------ *
 * FEATURE GATES
 *
 * Flip a flag to false and every mention of that offer disappears: the hero
 * button, its rung in "three ways to start", and the sticky bar. Nothing is left
 * behind advertising something that does not exist yet.
 *
 * Gating rather than deleting means the copy, the layout and the links stay in
 * the repository and stay type-checked, so turning it on later is one boolean
 * rather than a reconstruction.
 * ------------------------------------------------------------------ */

export const FEATURES = {
  /** The public demo server. Off until there is a host to point at. */
  demoServer: false,
  /** Patreon/YouTube members' server. */
  communityServer: true,
  /** Private dedicated instances for large groups. */
  privateInstances: true,
} as const;

export type Feature = keyof typeof FEATURES;

export function enabled(feature: Feature): boolean {
  return FEATURES[feature];
}

/**
 * The demo server. `FEATURES.demoServer` controls whether it is mentioned at
 * all; `address` is what it points at once it exists.
 */
export const DEMO_SERVER = {
  address: '',
} as const;

/** PLACEHOLDER — the Patreon tier that grants a seat on the community server. */
export const COMMUNITY_SERVER = {
  pending: true,
  tier: '',
  price: '',
} as const;

/* ------------------------------------------------------------------ *
 * WAYS IN
 *
 * Parallel to copy.ladder.rungs. Add a rung by adding copy and a row here;
 * the component maps over whichever is shorter, so a mismatch degrades to
 * fewer cards instead of a crash.
 * ------------------------------------------------------------------ */

export interface Rung {
  readonly accent: string;
  /** Appends the TBD marker to the rung's meta line. */
  readonly pending: boolean;
  /** Hidden entirely when this feature is off. Always shown when absent. */
  readonly feature?: Feature;
}

export const LADDER_RUNGS: readonly Rung[] = [
  { accent: 'var(--sp-cyan)', pending: true, feature: 'demoServer' },
  { accent: 'var(--sp-green)', pending: false },
  { accent: 'var(--sp-orange)', pending: true, feature: 'communityServer' },
  { accent: 'var(--sp-violet)', pending: true, feature: 'privateInstances' },
];

/* ------------------------------------------------------------------ *
 * APIS
 *
 * Three surfaces, two of which already publish generated reference docs into
 * gh-pages. The name, description and link for each are in
 * copy.integrations.apis; this supplies what is not editorial, in the same
 * order.
 * ------------------------------------------------------------------ */

export interface ApiSurface {
  /** Names the row. Not rendered; the copy entry at the same index is. */
  readonly id: 'clientWs' | 'serverApi' | 'serverWs';
  readonly spec: string;
  readonly accent: string;
  /** Set when the docs are not deployed yet. */
  readonly pending?: boolean;
}

export const APIS: readonly ApiSurface[] = [
  { id: 'clientWs', spec: 'AsyncAPI', accent: 'var(--sp-cyan)' },
  { id: 'serverApi', spec: 'OpenAPI', accent: 'var(--sp-green)' },
  // No generated docs deployed for this one yet: only docs/openapi.json and
  // docs/websocket-api.yaml exist in the repo.
  { id: 'serverWs', spec: 'AsyncAPI', accent: 'var(--sp-orange)', pending: true },
];

/* ------------------------------------------------------------------ *
 * PLATFORMS
 *
 * The cards themselves are copy.downloads.client and copy.downloads.server.
 * Their glyph and accent are in Downloads.astro, keyed on the `id` of each.
 *
 * Every one of them ships. release.yml builds build-client (Windows),
 * build-macos-client, build-linux-client, build-android, build-ios,
 * build-mod-fabric, build-mod-paper and build-mod-bds — so nothing there is
 * "coming soon". The old site still carried Coming Soon labels for macOS, iOS
 * and Linux long after they started shipping, which reads as a dead project.
 * ------------------------------------------------------------------ */

/**
 * The rotating list in "Your friend on ___ is in the conversation."
 *
 * Deliberately mixes devices with hosts, because both are the objection being
 * answered: "will it work for the person I actually play with, on the server we
 * actually use."
 */
export const FRIEND_TARGETS: readonly string[] = [
  'Switch',
  'Xbox',
  'PlayStation',
  'PC',
  'Android',
  'iPhone',
  'a Realm',
  'Aternos',
  'Fabric',
  'PaperMC',
];

/* ------------------------------------------------------------------ *
 * DISCORD ACCOUNT LINK
 *
 * /discord/callback is the redirect_uri registered with Discord and one of the
 * https links the client claims as a Universal/App Link. On iOS and Android the
 * OS usually opens the client before the page renders. On desktop the browser
 * renders it, and the page hands the payload to the custom scheme below, where
 * the deep-link plugin routes it to DiscordLinkService.
 *
 * The grant is implicit, so the access token arrives in the URL fragment and
 * never reaches a server. Nothing on the page may print it.
 * ------------------------------------------------------------------ */

/** The custom scheme the callback hands its payload to. */
export const DISCORD_DEEP_LINK = 'bedrock-voice-chat://discord-callback';

export const DISCORD_OUTCOMES = ['linked', 'cancelled', 'failed', 'idle'] as const;
export type DiscordOutcome = (typeof DISCORD_OUTCOMES)[number];

/**
 * The registry that assigns hostnames to entitled members. A local proving run
 * reaches the same name through a hosts entry, so this is a constant rather
 * than a build-time switch.
 */
export const REGISTRY_ORIGIN = 'https://registry.bedrockvoicechat.com';

/**
 * Outcomes of an enrollment return, in the order they ship in the HTML.
 *
 * `issued` is the only one that carries a token. The rest name why there is
 * none, and `taken` is the one that is not a failure: the claim redeemed
 * already, which is what a reloaded tab looks like.
 */
export const ENROLL_OUTCOMES = ['issued', 'taken', 'refused', 'idle'] as const;
export type EnrollOutcome = (typeof ENROLL_OUTCOMES)[number];

/**
 * `error=` values the registry redirects with, mapped to what each one means
 * for the person reading. An unlisted value falls back to the generic refusal
 * rather than rendering a raw code.
 */
export const ENROLL_REASONS: Readonly<Record<string, string>> = {
  not_entitled:
    'Your Discord account does not hold a qualifying membership. Join through Patreon or YouTube, wait for the role to sync, then try again.',
  already_registered:
    'That Discord account already holds an assigned name. One name is issued per member.',
  exchange_failed:
    'Discord did not complete the sign-in. Start again from the beginning.',
  internal: 'The registry could not finish. Try again in a few minutes.',
};

/**
 * The logo spectrum in mark order, violet to red. Values live in tokens.css;
 * this is the sequence, which is a fact about the mark rather than a design
 * decision made per component.
 */
export const SPECTRUM_STOPS: readonly string[] = [
  '--sp-violet',
  '--sp-indigo',
  '--sp-blue',
  '--sp-azure',
  '--sp-cyan',
  '--sp-teal',
  '--sp-aqua',
  '--sp-mint',
  '--sp-green',
  '--sp-grass',
  '--sp-lime',
  '--sp-yellow',
  '--sp-amber',
  '--sp-orange',
  '--sp-ember',
  '--sp-red',
  '--sp-crimson',
];

/* ------------------------------------------------------------------ */

export const AUDIENCES = ['player', 'operator', 'creator'] as const;
export type Audience = (typeof AUDIENCES)[number];

export const DEFAULT_AUDIENCE: Audience = 'player';

/** How long each track holds before the hero rotates to the next one. */
export const AUDIENCE_ROTATE_MS = 3000;

export interface AudienceMeta {
  readonly id: Audience;
  /** CSS custom property holding this audience's spectrum stop. */
  readonly accent: string;
  /**
   * Where clicking this takes you.
   *
   * The switcher navigates as well as switching. In a full-height hero, a
   * control that only rewrites copy two inches above itself gives no sign that
   * it did anything, and the page below is off-screen. Moving the reader is the
   * feedback.
   */
  readonly anchor: string;
}

export const AUDIENCE_META: readonly AudienceMeta[] = [
  { id: 'player', accent: 'var(--aud-player)', anchor: 'download' },
  { id: 'operator', accent: 'var(--aud-operator)', anchor: 'operators' },
  { id: 'creator', accent: 'var(--aud-creator)', anchor: 'creators' },
];

/** The section each audience jumps to. */
export function anchorFor(id: Audience): string {
  return AUDIENCE_META.find((a) => a.id === id)?.anchor ?? '';
}

export function isAudience(value: unknown): value is Audience {
  return typeof value === 'string' && (AUDIENCES as readonly string[]).includes(value);
}
