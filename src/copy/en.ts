import { VERSION } from '../lib/site';
import type { SiteCopy } from './types';

/**
 * English. The reference locale.
 *
 * `satisfies SiteCopy` rather than `: SiteCopy` so the exported object keeps its
 * literal types (autocomplete on every key) while still failing the build if a
 * key is missing or misspelled.
 *
 * Notes for translators:
 *  - Headings are split into `lead` and `strong`. The `strong` half renders bold.
 *    Put the emphasis where it belongs in your language; it does not have to be
 *    the second half of the sentence.
 *  - No HTML in any string. If a sentence needs a link, the `href` sits on the
 *    same object as its label.
 *  - Copy the `href` values across unchanged. They are here so that a label and
 *    its destination can be read as one fact, not so that each language picks
 *    its own targets. Override one only where the destination genuinely differs.
 *  - `tbd` marks a value that is not real yet and will be removed before launch.
 *
 * `VERSION` is the only interpolation. It comes from the build environment so a
 * release does not need a commit here.
 */

/** Where the source and its release artefacts live. */
const REPO = 'https://github.com/Alaydriem/bedrock-voice-chat';

export const en = {
  meta: {
    title: 'Bedrock Voice Chat · proximity voice for Minecraft Bedrock',
    description:
      'Positional, encrypted, self-hosted voice chat for Minecraft Bedrock and Java. Windows, macOS, Linux, Android and iOS. Console players join from the mobile app.',
  },

  common: {
    skipToContent: 'Skip to content',
    tbd: 'TBD',
  },

  nav: {
    menu: 'Menu',
    ariaLabel: 'Main',
    links: [
      { label: 'How it works', href: '#how' },
      { label: 'Platforms', href: '#download' },
      { label: 'Run a server', href: '#operators' },
      { label: 'Wiki', href: '/wiki/' },
      { label: 'Integrations', href: '#integrations' },
    ],
    discord: { label: 'Discord', href: 'https://discord.gg/CdtchD5zxr', external: true },
    download: { label: 'Download', href: '#download' },
  },

  hero: {
    eyebrow: 'Proximity voice · Minecraft Bedrock',
    ringLabel: 'Three players talking, their voices rising and falling as they move around you',
    tracks: {
      player: {
        heading: { lead: 'Walk up to someone.', strong: "You're already talking." },
        lead: "There's no call to join and no key to hold. You talk, and whoever is close enough hears you. Walk off mid-sentence and your voice fades out, the same way it would anywhere else.",
        cta: { label: 'Download for Windows', href: '#download' },
      },
      operator: {
        heading: { lead: 'Five minutes, and', strong: 'your world has voice.' },
        lead: "Add one addon to your world and run one binary next to your server. It's free, you host it yourself, and the source is public, and your world files stay exactly as they are.",
        cta: { label: 'Read the install guide', href: '/wiki/server/installation/' },
      },
      creator: {
        heading: { lead: 'Every voice lands on', strong: 'its own track.' },
        lead: 'Record a session and you get one timecoded track per player. Sort the levels out afterwards instead of hoping they were right at the time. A Discord call with Craig gives you a single mixed track and no way back.',
        cta: { label: 'See the creator tools', href: '#creators' },
      },
    },
    demoCta: { label: 'Try the demo server', href: '#ladder' },
    creatorAside:
      "Multitrack recording is in every build, including the free self-hosted one. There's no creator tier to buy.",
  },

  switcher: {
    label: "I'm here to",
    ariaLabel: 'Jump to what matters to you',
    options: {
      player: { label: 'I play on Bedrock', destination: 'Get the app' },
      operator: { label: 'I run a server', destination: 'Add it to my world' },
      creator: { label: 'I make content', destination: 'Record a session' },
    },
  },

  consoleClaim: {
    eyebrow: 'The part no Java mod can do',
    heading: { before: 'Your friend on', after: 'is in the conversation.' },
    body: "Bedrock Voice Chat supports every platform that runs Minecraft Bedrock via the companion app that runs on PC, Mac, Linux, iOS, and Android. Console players use the mobile app, and everyone can manage their audio settings from within Minecraft. No tabbing between windows or apps, and no one is left out.",
  },

  capabilities: {
    compatibility: {
      eyebrow: 'Compatibility',
      heading: { lead: 'It works with', strong: 'the world you already have.' },
      body: "Install the Bedrock Addon for your world, the no-net Addon for your Realm or Aternos Server, or the Java mod/plugin for Fabric or PaperMC.",
      facts: [
        'Bedrock Dedicated Server, Aternos, and any host you control',
        'Realms works through the client.',
        'Java: Fabric and Paper, from Modrinth',
        'Self-hosted and source available.',
      ],
      captionKey: '',
      captionValue: '',
      vizLabel:
        'Support matrix covering Bedrock Dedicated Server, Aternos and other hosts, Realms, console, Fabric, Paper, the Rich API and Simple Voice Chat',
    },
    proximity: {
      eyebrow: 'Proximity',
      heading: { lead: 'You get louder by ', strong: 'walking closer.' },
      body: 'Volume drops off along a curve you configure, so nobody snaps in and out at a hard edge. Whisper to the person beside you, shout across a build site, or watch a whole base as a spectator without anyone hearing you.',
      facts: [
        'Configurable range and falloff curve, per server',
        'Crouch to enter whisper mode for close range only',
        'Spectator mode hears everything nearby, is heard by nobody'
      ],
      captionKey: 'Your range',
      captionValue: '24 M · 3 IN RANGE',
      vizLabel:
        'A radial showing three speakers at different distances, each returning a coloured ping',
    },
    channels: {
      eyebrow: 'Channels',
      heading: { lead: 'Split off into', strong: 'your own channel.' },
      body: "Taking on a trial chamber or the deep dark? Coordinating a build? Create a channel for you and your team to talk in, and nobody else can hear you, but you can still hear the world around you. Channels are persistent, so you can leave and come back without losing your place.",
      facts: [
        'Persistent groups that survive relogs',
        'Per-player volume from 0 to 150%',
        'Mute and deafen without leaving',
      ],
      captionKey: 'Channel',
      captionValue: 'FULL VOLUME · ANY DISTANCE',
      vizLabel: 'Two panels: a proximity group and a private squad channel',
    },
    recording: {
      eyebrow: 'For creators',
      heading: { lead: 'Record everyone.', strong: 'On separate tracks.' },
      body: 'Everyone gets their own timecoded track. Turn one person down, cut another out, and it all still lines up against your video.',
      facts: [
        'One track per player, timecode-aligned',
        'Stream Deck plugin on the Elgato Marketplace',
        'WebSocket API for anything else you automate',
        'One OBS output for Streamers and Content Creators'
      ],
      captionKey: 'Session',
      captionValue: '3 TRACKS · TIMECODED',
      vizLabel: 'Three separate recording lanes, one per player, scrolling under a playhead',
    },
  },

  proximityStats: [
    { value: '24 Blocks', label: 'Default range, configurable per server' },
    { value: '0–150%', label: 'Per-player volume, set by each listener' },
    { value: 'Whisper', label: 'Short-range mode for close conversation' },
    { value: 'Spectator', label: 'Hears everything, is heard by nobody' },
  ],

  videos: {
    eyebrow: 'See it working',
    heading: { lead: 'Easier to watch', strong: 'than to read about.' },
    playPrefix: 'Play',
    items: [
      {
        title: 'What Bedrock Voice Chat is',
        blurb: 'See the original announcement video showcasing what Bedrock Voice Chat is',
        kind: 'Announcement',
      },
      {
        title: 'Setting up a server',
        blurb: 'Learn how to setup Bedrock Voice Chat Server and install the addon to your world.',
        kind: 'Setup guide',
      },
    ],
  },

  comparison: {
    eyebrow: '',
    heading: { lead: 'Why not', strong: 'use Discord?' },
    lead: "Bedrock Voice Chat is designed for Minecraft, and does things no other call app can do.",
    capabilityColumn: 'Capability',
    columns: ['BVC', 'Discord call', 'Simple Voice Chat'],
    rows: [
      {
        capability: 'Voice positioned in the world',
        note: 'Volume follows how far apart the two of you are standing.',
      },
      {
        capability: 'Works on Bedrock',
        note: 'Simple Voice Chat is Java only. Discord works, but it sits alongside the game.',
      },
      {
        capability: 'Console players included',
        note: 'They run the mobile app and get placed in the world like everyone else.',
      },
      {
        capability: 'Separate recording track per player',
        note: 'A call gives you one mixed track and no way to separate it later.',
      },
      { capability: 'Group channels that ignore distance' },
      { capability: 'Runs on hardware you control' },
      { capability: 'Free' },
    ],
    reads: { yes: 'Yes', partial: 'Partly', no: 'No' },
  },

  ladder: {
    eyebrow: 'Three ways in',
    heading: { lead: 'Three ways', strong: 'to start.' },
    rungs: [
      {
        label: 'Hear it first',
        title: 'The demo server',
        body: "Install the client, put in one address, and walk around with whoever's on. Nothing to configure and nothing to host. It's the quickest way to hear what this page is describing.",
        meta: 'Coming soon',
        cta: { label: 'Get the client', href: '#download' },
      },
      {
        label: 'Run it yourself',
        title: 'Your own server',
        body: "One addon on the world, one server binary, one config file. It's free and it's yours. No seat count, no per-player billing, and no account system to sign anyone up for.",
        meta: 'Free · self-hosted',
        cta: { label: 'Install guide', href: '/wiki/' },
      },
      {
        label: 'Become a Member',
        title: 'Patreon and Youtube Members Server',
        body: "Become a member and link your current world to a members server, included with your membership.",
        meta: 'Coming Soon',
        cta: { label: 'See tiers', href: 'https://www.patreon.com/c/Alaydriem', external: true },
      },
      {
        label: 'Your Own Hosted Instance',
        title: 'Private Servers',
        body: "Need your own dedicated server for a large group or event? This tier will give you a private instance of the server with dedicated resources and support.",
        meta: 'Coming Soon',
        cta: { label: 'See tiers', href: '#' },
      },
    ],
  },

  downloads: {
    eyebrow: 'Download',
    heading: { lead: 'Every platform.', strong: 'Every device.' },
    getLabel: 'Get it',
    specs: [
      'Encrypted with QUIC and mutual TLS',
      '41 ms typical end-to-end latency',
      'Opus codec, tuned for voice at low bitrate',
    ],
    note: 'On Xbox, PlayStation or Switch? Install the mobile app on your phone and sign in with the same account.',
    serverSubhead: 'For your server',
    channels: {
      stable: 'Full release',
      beta: 'Open beta',
      testflight: 'TestFlight',
    },
    client: [
      {
        id: 'windows',
        os: 'Windows',
        note: 'ASIO + WASAPI',
        channel: 'stable',
        href: 'https://www.bedrockvoicechat.com/downloads/latest/windows.exe',
      },
      {
        id: 'macos',
        os: 'macOS',
        note: 'TestFlight',
        channel: 'testflight',
        // One TestFlight build covers both macOS and iOS.
        href: 'https://testflight.apple.com/join/JSG7bVqC',
      },
      {
        id: 'linux',
        os: 'Linux',
        note: 'deb + AppImage',
        channel: 'stable',
        href: `${REPO}/releases/tag/v${VERSION}`,
      },
      {
        id: 'android',
        os: 'Android',
        note: 'Google Play',
        channel: 'beta',
        href: 'https://play.google.com/store/apps/details?id=com.alaydriem.bvc.client',
      },
      {
        id: 'ios',
        os: 'iOS',
        note: 'TestFlight',
        channel: 'testflight',
        href: 'https://testflight.apple.com/join/JSG7bVqC',
      },
    ],
    // One card per download, not one per platform. Fabric and Paper were two
    // cards pointing at the same Modrinth page, which reads as two downloads.
    server: [
      {
        id: 'bds',
        os: 'Bedrock Dedicated Server',
        note: 'CurseForge',
        channel: 'stable',
        href: 'https://www.curseforge.com/minecraft-bedrock/addons/bedrock-voice-chat',
      },
      {
        id: 'realms',
        os: 'Realms & Aternos',
        note: 'CurseForge',
        channel: 'stable',
        // The no-net variant, for any host without @minecraft/server-net.
        href: 'https://www.curseforge.com/minecraft-bedrock/addons/bedrock-voice-chat-for-realms',
      },
      {
        id: 'java',
        os: 'Java · Fabric & Paper',
        note: 'Modrinth',
        channel: 'stable',
        // One Modrinth page carries both the Fabric mod and the Paper plugin.
        href: 'https://modrinth.com/mod/bedrock-voice-chat',
      },
    ],
  },

  operators: {
    eyebrow: 'For server operators',
    heading: { lead: 'Five minutes,', strong: 'start to finish.' },
    steps: [
      {
        title: 'Run Bedrock Voice Chat Server',
        detail:
          'Run a single docker command and be up and running in minutes.',
        time: '~5 min',
      },
      {
        title: 'Install the addon on your world',
        detail:
          'Install an .mcpack for Bedrock Dedicated Server, a .mcpack for Realms and Aternos Servers, a mod for Fabric, or a plugin for PaperMC with 2 config file edits.',
        time: '~1 min',
      },
      {
        title: 'Sign in',
        detail:
          'You and your players open the client, enter the address, and sign in with the account you already play on. Players have nothing to setup, then run Bedrock Voice Chat in the background.',
        time: '~1 min',
      },
    ],
    note: "Not the person who runs your server? Send them this section. That's the whole job.",
    actions: {
      guide: { label: 'Read the install guide', href: '/wiki/server/installation/' },
      // Every CurseForge project on one page, because this one button stands
      // for both the Bedrock addon and the Realms/no-net variant.
      bedrock: {
        label: 'Bedrock addon',
        href: 'https://www.curseforge.com/members/alaydriem/projects',
        external: true,
      },
      java: { label: 'Java plugin', href: 'https://modrinth.com/mod/bedrock-voice-chat', external: true },
    },
  },

  integrations: {
    eyebrow: 'Integrations',
    heading: { lead: 'Bind it to', strong: 'real buttons.' },
    deck: {
      label: 'Stream Deck',
      heading: 'Mute, deafen and record without leaving the game.',
      body: 'The official plugin on the Elgato Marketplace. Mute, deafen and arm a multitrack recording from a physical key, and read the state off the key face without switching windows.',
      cta: {
        label: 'Get it on the Elgato Marketplace',
        href: 'https://marketplace.elgato.com/product/bedrock-voice-chat-c5a151d6-3669-487f-9548-bfe689e50203',
        external: true,
      },
      ariaLabel:
        'The Bedrock Voice Chat Stream Deck plugin: a Stream Deck with mute, volume and record keys bound to it',
    },
    apisLabel: 'Or build your own',
    // CI publishes each generated site under /api/<version> and
    // /websocket/<version>, with an unversioned alias at the directory root.
    apis: [
      {
        name: 'Client WebSocket',
        what: 'Drive the desktop client: mute, deafen, record, subscribe to state. This is what the Stream Deck plugin talks to.',
        href: '/websocket',
      },
      {
        name: 'Server API',
        what: 'Channels, players, permissions and moderation over HTTP with mTLS.',
        href: '/api',
      },
      {
        name: 'Server WebSocket',
        what: 'Live server-side events: joins, leaves, channel changes and position feeds.',
        // No generated docs deployed for this one yet: only docs/openapi.json
        // and docs/websocket-api.yaml exist in the repo. Repoint once CI
        // publishes it; APIS marks it pending until then.
        href: '/websocket',
      },
    ],
    wikiCta: { label: 'Read the wiki', href: '/wiki/' },
    pendingDocs: 'docs path',
  },

  faq: {
    eyebrow: 'Questions',
    heading: { lead: 'The things', strong: 'people ask first.' },
    entries: [
      {
        q: 'Do I need to run a server?',
        a: "Someone does, but it doesn't have to be you. If your server already runs BVC you only need the client. If nobody runs it, the install is one addon plus one docker command and takes about five minutes.",
        link: { label: 'Install guide', href: '/wiki/server/installation/' },
      },
      {
        q: 'Is it free?',
        a: 'Yes. Self-hosting is free, with no seat count and no per-player billing. Nothing is held back behind a paid tier either, so multitrack recording is in every build. The Patreon tier is only there if you would sooner not host anything and want a seat on a server that is already up.',
      },
      {
        q: 'My friend plays on Xbox / PlayStation / Switch. Can they use it?',
        a: 'Yes, and this is the part no Java-only mod can do. They run the mobile app next to the console and sign in with the same account, then join their server.',
        link: { label: 'Console and mobile', href: '/wiki/' },
      },
      {
        q: 'Does it work on Realms?',
        a: "Yes. Realms is supported by the no-net addon. Run BVC Server, and connect to your Realm from the app.",
      },
      {
        q: 'Does it work on Aternos and other hosts?',
        a: 'Yes. Aternos is supported by the no-net addon. Run BVC Server, and connect to your server from the app.',
      },
      {
        q: 'Do you record or store my voice?',
        a: 'No. The server never captures audio on its own, and there is no moderation feature listening for you. Recording is something a person starts, and it stays visible while it runs. On a self-hosted install the audio never reaches us at all.',
      },
      {
        q: 'Does it work with Java?',
        a: 'Yes, on Fabric and Paper, from Modrinth.',
        link: {
          label: 'Get the Java plugin',
          href: 'https://modrinth.com/mod/bedrock-voice-chat',
          external: true,
        },
      },
      {
        q: 'Is Geyser / Floodgate supported?',
        a: 'Yes, on Fabric and Paper, from Modrinth. Bedrock Voice Chat runs on both Bedrock and Java servers. And everyone can talk to each other.',
        link: {
          label: 'Get the Java plugin',
          href: 'https://modrinth.com/mod/bedrock-voice-chat',
          external: true,
        },
      },
      {
        q: 'Do my players need to create an account?',
        a: "No. They sign in with the Microsoft account they already play on. Your login information is used solely to identify you and connect you to Minecraft.",
      }
    ],
  },

  wiki: {
    eyebrow: 'Documentation',
    heading: { lead: 'Have questions?', strong: '' },
    lead: 'Full documentation covering setup, configuration and troubleshooting.',
    entries: [],
    browse: { label: 'Browse the wiki', href: '/wiki/' },
    apiReference: { label: 'API reference', href: '#integrations' },
  },

  stickyCta: {
    messages: {
      player: '',
      playerNoDemo: 'Free, self-hosted, and every platform is supported. ',
      operator: '',
      creator: 'Every voice on its own timecoded track.',
    },
    demo: { label: '', href: '#ladder' },
    download: { label: 'Download', href: '#download' },
    installGuide: { label: 'Install guide', href: '/wiki/server/installation/' },
  },

  discordCallback: {
    meta: {
      title: 'Discord link · Bedrock Voice Chat',
      description: 'Finishes a Discord account link started in Bedrock Voice Chat.',
    },
    from: 'Discord',
    outcomes: {
      linked: {
        heading: { lead: 'Switch back to', strong: 'Bedrock Voice Chat' },
        body: 'Discord sent your account back. Bedrock Voice Chat finishes the link when you switch to it.',
        action: { label: 'Reopen Bedrock Voice Chat', href: 'bedrock-voice-chat://discord-callback' },
      },
      cancelled: {
        heading: { lead: 'Discord link', strong: 'cancelled' },
        body: 'You declined the request on Discord. Nothing changed on either account. Open Settings in Bedrock Voice Chat to link Discord again.',
        action: { label: 'Reopen Bedrock Voice Chat', href: 'bedrock-voice-chat://discord-callback' },
      },
      failed: {
        heading: { lead: 'Discord did not', strong: 'finish the link' },
        body: 'Discord returned an error instead of an account. Open Settings in Bedrock Voice Chat and link Discord again.',
        action: { label: 'Reopen Bedrock Voice Chat', href: 'bedrock-voice-chat://discord-callback' },
        secondary: { label: 'Open the wiki', href: '/wiki/' },
      },
      idle: {
        heading: { lead: 'Nothing to', strong: 'link here' },
        body: 'This page finishes a Discord link that starts inside Bedrock Voice Chat. Open Settings in the app and link Discord.',
        action: { label: 'Get Bedrock Voice Chat', href: '/#download' },
        secondary: { label: 'Open the wiki', href: '/wiki/' },
      },
    },
    codeLabel: 'Discord reported',
    footnote:
      'Linking your Discord account to Bedrock Voice Chat enables you to receive experimental features and participate in the community. You can unlink Discord at any time from the app settings.',
    home: { label: 'bedrockvoicechat.com', href: '/' },
  },

  enrolled: {
    meta: {
      title: 'Enrollment · Bedrock Voice Chat',
      description: 'Finishes an enrollment started at the Bedrock Voice Chat registry.',
    },
    outcomes: {
      issued: {
        heading: { lead: 'Your server has an', strong: 'enrollment token' },
        body: 'Paste it into your server config and start the server. It works once, and it expires in a day.',
        action: { label: 'Open the setup guide', href: '/wiki/server/' },
      },
      taken: {
        heading: { lead: 'This link has', strong: 'already been used' },
        body: 'An enrollment token is handed over once. If you did not copy it, start again and you will be issued another.',
        action: {
          label: 'Enroll again',
          href: 'https://registry.bedrockvoicechat.com/oauth/start',
          external: true,
        },
        secondary: { label: 'Open the wiki', href: '/wiki/' },
      },
      refused: {
        heading: { lead: 'No token was', strong: 'issued' },
        body: 'The registry refused this enrollment.',
        action: {
          label: 'Enroll again',
          href: 'https://registry.bedrockvoicechat.com/oauth/start',
          external: true,
        },
        secondary: { label: 'Open the wiki', href: '/wiki/' },
      },
      idle: {
        heading: { lead: 'Nothing to', strong: 'enroll here' },
        body: 'This page finishes an enrollment that starts at the registry. Sign in with Discord to be issued a name.',
        action: {
          label: 'Start enrollment',
          href: 'https://registry.bedrockvoicechat.com/oauth/start',
          external: true,
        },
        secondary: { label: 'Open the wiki', href: '/wiki/' },
      },
    },
    tokenLabel: 'Enrollment token',
    instructions: 'server { enrollment { token = "..." } }',
    copyAction: 'Copy',
    copied: 'Copied',
    redeeming: 'Redeeming...',
    footnote:
      'The token identifies your server to the registry once. The name you are assigned stays with your server key, and the registry keeps a record of which Discord account it was issued to.',
    home: { label: 'bedrockvoicechat.com', href: '/' },
  },

  footer: {
    blurb:
      'Proximity voice chat for Minecraft Bedrock and Java. Self-hosted, encrypted, source available.',
    columns: [
      {
        title: 'Download',
        links: [
          {
            label: 'Windows',
            href: 'https://www.bedrockvoicechat.com/downloads/latest/windows.exe',
            external: true,
          },
          { label: 'macOS & iOS', href: 'https://testflight.apple.com/join/JSG7bVqC', external: true },
          {
            label: 'Android',
            href: 'https://play.google.com/store/apps/details?id=com.alaydriem.bvc.client',
            external: true,
          },
          { label: 'Linux', href: `${REPO}/releases/tag/v${VERSION}`, external: true },
          {
            label: 'Bedrock addon',
            href: 'https://www.curseforge.com/members/alaydriem/projects',
            external: true,
          },
          { label: 'Java plugin', href: 'https://modrinth.com/mod/bedrock-voice-chat', external: true },
        ],
      },
      {
        title: 'Docs',
        links: [
          { label: 'Wiki', href: '/wiki/' },
          {
            label: 'Stream Deck plugin',
            href: 'https://marketplace.elgato.com/product/bedrock-voice-chat-c5a151d6-3669-487f-9548-bfe689e50203',
            external: true,
          },
          { label: 'Client WebSocket API', href: '/websocket' },
          { label: 'Server API', href: '/api' },
        ],
      },
      {
        title: 'Community',
        links: [
          { label: 'Discord', href: 'https://discord.gg/CdtchD5zxr', external: true },
          { label: 'YouTube', href: 'https://youtube.com/@alaydriem', external: true },
          { label: 'Patreon', href: 'https://www.patreon.com/c/Alaydriem', external: true },
          { label: 'GitHub', href: REPO, external: true },
        ],
      },
    ],
    copyright: (year: number) => `© ${year} Alaydriem`,
    source: 'source available on GitHub',
  },
} satisfies SiteCopy;
