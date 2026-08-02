/**
 * Everything about the site that is a fact rather than a design decision.
 * Change a URL or a version here and it changes everywhere.
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

const REPO = 'https://github.com/Alaydriem/bedrock-voice-chat';

export const LINKS = {
  repo: REPO,
  releases: `${REPO}/releases`,
  releaseTag: `${REPO}/releases/tag/v${VERSION}`,
  modsRelease: `${REPO}/releases/tag/${MODS_VERSION}`,
  wiki: '/wiki/',
  install: '/wiki/server/installation/',
  discord: 'https://discord.gg/CdtchD5zxr',
  youtube: 'https://youtube.com/@alaydriem',
  patreon: 'https://www.patreon.com/c/Alaydriem',
  /** Served off the apex, not from gh-pages. */
  windowsDirect: 'https://www.bedrockvoicechat.com/downloads/latest/windows.exe',
  googlePlay: 'https://play.google.com/store/apps/details?id=com.alaydriem.bvc.client',
  /** One TestFlight build covers both macOS and iOS. */
  testFlight: 'https://testflight.apple.com/join/JSG7bVqC',
  modrinth: 'https://modrinth.com/mod/bedrock-voice-chat',
  curseforgeBedrock: 'https://www.curseforge.com/minecraft-bedrock/addons/bedrock-voice-chat',
  streamDeck:
    'https://marketplace.elgato.com/product/bedrock-voice-chat-c5a151d6-3669-487f-9548-bfe689e50203',
} as const;

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
  readonly href: string;
  readonly external: boolean;
  /** Appends the TBD marker to the rung's meta line. */
  readonly pending: boolean;
  /** Hidden entirely when this feature is off. Always shown when absent. */
  readonly feature?: Feature;
}

export const LADDER_RUNGS: readonly Rung[] = [
  { accent: 'var(--sp-cyan)', href: '#download', external: false, pending: true, feature: 'demoServer' },
  { accent: 'var(--sp-green)', href: '/wiki/', external: false, pending: false },
  { accent: 'var(--sp-orange)', href: 'https://www.patreon.com/c/Alaydriem', external: true, pending: true, feature: 'communityServer' },
  { accent: 'var(--sp-violet)', href: 'https://www.patreon.com/c/Alaydriem', external: true, pending: true, feature: 'privateInstances' },
];

/* ------------------------------------------------------------------ *
 * APIS
 *
 * Three surfaces, two of which already publish generated reference docs into
 * gh-pages. Each generated site is versioned by CI (`/api/<version>`,
 * `/websocket/<version>`) with an unversioned alias at the root of its
 * directory, which is what these link to.
 * ------------------------------------------------------------------ */

export interface ApiSurface {
  /** Joins to the matching entry in copy.integrations.apis. */
  readonly id: 'clientWs' | 'serverApi' | 'serverWs';
  readonly spec: string;
  readonly href: string;
  readonly accent: string;
  /** Set when the docs are not deployed yet. */
  readonly pending?: boolean;
}

export const APIS: readonly ApiSurface[] = [
  { id: 'clientWs', spec: 'AsyncAPI', href: '/websocket', accent: 'var(--sp-cyan)' },
  { id: 'serverApi', spec: 'OpenAPI', href: '/api', accent: 'var(--sp-green)' },
  // No generated docs deployed for this one yet: only docs/openapi.json and
  // docs/websocket-api.yaml exist in the repo. Repoint once CI publishes it.
  { id: 'serverWs', spec: 'AsyncAPI', href: '/websocket', accent: 'var(--sp-orange)', pending: true },
];

/* ------------------------------------------------------------------ *
 * PLATFORMS
 *
 * Every one of these ships. release.yml builds build-client (Windows),
 * build-macos-client, build-linux-client, build-android, build-ios,
 * build-mod-fabric, build-mod-paper and build-mod-bds — so nothing here is
 * "coming soon". The old site still carried Coming Soon labels for macOS, iOS
 * and Linux long after they started shipping, which reads as a dead project.
 * ------------------------------------------------------------------ */

export interface Platform {
  /** A product name; not translated. */
  readonly os: string;
  /** Where the build comes from. Store names, so not translated. */
  readonly note: string;
  /** Joins to copy.downloads.channels. */
  readonly channel: 'stable' | 'beta' | 'testflight';
  readonly accent: string;
  readonly href: string;
}

export const CLIENT_PLATFORMS: readonly Platform[] = [
  {
    os: 'Windows',
    note: 'ASIO + WASAPI',
    channel: 'stable',
    accent: 'var(--sp-violet)',
    href: LINKS.windowsDirect,
  },
  {
    os: 'macOS',
    note: 'TestFlight',
    channel: 'testflight',
    accent: 'var(--sp-blue)',
    href: LINKS.testFlight,
  },
  {
    os: 'Linux',
    note: 'deb + AppImage',
    channel: 'stable',
    accent: 'var(--sp-cyan)',
    href: LINKS.releaseTag,
  },
  {
    os: 'Android',
    note: 'Google Play',
    channel: 'beta',
    accent: 'var(--sp-green)',
    href: LINKS.googlePlay,
  },
  {
    os: 'iOS',
    note: 'TestFlight',
    channel: 'testflight',
    accent: 'var(--sp-orange)',
    href: LINKS.testFlight,
  },
];

export const SERVER_PLATFORMS: readonly Platform[] = [
  {
    os: 'Bedrock Dedicated Server',
    note: 'CurseForge',
    channel: 'stable',
    accent: 'var(--sp-lime)',
    href: LINKS.curseforgeBedrock,
  },
  {
    os: 'Java · Fabric',
    note: 'Modrinth',
    channel: 'stable',
    accent: 'var(--sp-yellow)',
    href: LINKS.modrinth,
  },
  {
    os: 'Java · Paper',
    note: 'Modrinth',
    channel: 'stable',
    accent: 'var(--sp-ember)',
    href: LINKS.modrinth,
  },
];

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
