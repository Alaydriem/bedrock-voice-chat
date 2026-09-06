import type { Audience, DiscordOutcome, EnrollOutcome } from '../lib/site';

/**
 * The contract every locale has to satisfy.
 *
 * Words AND their link targets live here. Colours, glyphs, feature gates and
 * anything else structural stay in lib/site.ts.
 *
 * The link targets used to live in lib/site.ts too, on the theory that a URL
 * cannot be mistranslated if there are no URLs to mistranslate. In practice it
 * meant no one could answer "where does this button go" without opening three
 * files, and two of the joins were by array index, so a copy edit could silently
 * repoint a link. A label and its href are one fact and now sit as one object.
 *
 * The cost is that a new locale carries the hrefs too. Copying them unchanged is
 * the correct default; a locale only overrides one where the destination really
 * differs by language.
 *
 * Locales are typed against this interface, so a missing or misspelled key is a
 * build failure rather than an `undefined` rendered into the page. Adding a
 * string to the site means adding it here first, and every language then fails
 * to compile until it is filled in. That is deliberate: silent English fallback
 * is how half-translated sites happen.
 */

/**
 * A link: the words on it and where it goes, together.
 *
 * `external` decides `target="_blank" rel="noopener"`. Set it on anything off
 * this origin. Leave it off for `/paths`, `#anchors` and custom schemes, which
 * must stay in the current tab.
 */
export interface LinkCopy {
  readonly label: string;
  readonly href: string;
  readonly external?: boolean;
}

/**
 * A heading split into its normal-weight and bold parts.
 *
 * Not a string with markup in it. Bold placement differs per language, and
 * embedding `<b>` in translatable text means translators editing HTML.
 */
export interface Heading {
  readonly lead: string;
  readonly strong: string;
}

export interface CtaCopy {
  readonly label: string;
}

export interface HeroTrack {
  readonly heading: Heading;
  readonly lead: string;
  readonly cta: LinkCopy;
}

export interface AudienceOption {
  readonly label: string;
  /** Names where the button goes, so it does not read as a filter. */
  readonly destination: string;
}

export interface Fact {
  readonly text: string;
}

export interface CapabilityCopy {
  readonly eyebrow: string;
  readonly heading: Heading;
  readonly body: string;
  readonly facts: readonly string[];
  readonly captionKey: string;
  readonly captionValue: string;
  /** Alt text for the canvas visual. */
  readonly vizLabel: string;
}

export interface StatCopy {
  readonly value: string;
  readonly label: string;
}

export interface VideoCopy {
  readonly title: string;
  readonly blurb: string;
  readonly kind: string;
}

export interface ComparisonRow {
  readonly capability: string;
  readonly note?: string;
}

export interface RungCopy {
  readonly label: string;
  readonly title: string;
  readonly body: string;
  readonly meta: string;
  readonly cta: LinkCopy;
}

export interface StepCopy {
  readonly title: string;
  readonly detail: string;
  readonly time: string;
}

export interface FaqEntry {
  readonly q: string;
  readonly a: string;
  /** Omitted where the answer is the whole answer. */
  readonly link?: LinkCopy;
}

export interface WikiEntry {
  readonly who: string;
  readonly title: string;
  readonly blurb: string;
  readonly href: string;
}

export interface ApiCopy {
  readonly name: string;
  readonly what: string;
  readonly href: string;
}

/** Which build a download is. Joins to `downloads.channels`. */
export type ReleaseChannel = 'stable' | 'beta' | 'testflight';

/** One card in a download grid. */
export interface PlatformCopy {
  /**
   * Joins to the glyph and accent in Downloads.astro. Never translated, and
   * never reused between the two grids — that pairing is what survives a copy
   * edit to `os`.
   */
  readonly id: string;
  readonly os: string;
  /** Where it comes from: a store name or a package format. */
  readonly note: string;
  readonly href: string;
  readonly channel: ReleaseChannel;
}

export interface SectionHead {
  readonly eyebrow: string;
  readonly heading: Heading;
  readonly lead?: string;
}

/** The <title> and meta description of one page. */
export interface MetaCopy {
  readonly title: string;
  readonly description: string;
}

/**
 * One way the Discord return can land.
 *
 * All four ship in the HTML and the page reveals whichever the callback URL
 * turned out to describe, so the outcome needs no round trip and survives a
 * script that never runs.
 */
export interface OutcomeCopy {
  readonly heading: Heading;
  readonly body: string;
  readonly action: LinkCopy;
  /** Left out where one action is the whole answer. */
  readonly secondary?: LinkCopy;
}

/** One column of the site footer. */
export interface FooterColumn {
  readonly title: string;
  readonly links: readonly LinkCopy[];
}

export interface SiteCopy {
  readonly meta: MetaCopy;

  readonly common: {
    readonly skipToContent: string;
    /** Marks a value that is not real yet. */
    readonly tbd: string;
  };

  readonly nav: {
    readonly menu: string;
    readonly ariaLabel: string;
    /** Rendered in order. */
    readonly links: readonly LinkCopy[];
    readonly discord: LinkCopy;
    readonly download: LinkCopy;
  };

  readonly hero: {
    readonly eyebrow: string;
    readonly tracks: Readonly<Record<Audience, HeroTrack>>;
    /**
     * `href` is the fallback shown while there is no demo server. Once
     * DEMO_SERVER.address is set the component swaps in a `bvc://` link.
     */
    readonly demoCta: LinkCopy;
    readonly ringLabel: string;
    readonly creatorAside: string;
  };

  readonly switcher: {
    readonly label: string;
    readonly ariaLabel: string;
    /**
     * These are buttons, not links: they set the audience state the hero and
     * the sticky rail read, and the scroll target lives with that state in
     * AUDIENCE_META.
     */
    readonly options: Readonly<Record<Audience, AudienceOption>>;
  };

  readonly consoleClaim: {
    readonly eyebrow: string;
    readonly heading: {
      readonly before: string;
      readonly after: string;
    };
    readonly body: string;
  };

  readonly capabilities: {
    readonly compatibility: CapabilityCopy;
    readonly proximity: CapabilityCopy;
    readonly channels: CapabilityCopy;
    readonly recording: CapabilityCopy;
  };

  readonly proximityStats: readonly StatCopy[];

  readonly videos: SectionHead & {
    readonly items: readonly VideoCopy[];
    readonly playPrefix: string;
  };


  readonly comparison: SectionHead & {
    readonly capabilityColumn: string;
    readonly columns: readonly string[];
    readonly rows: readonly ComparisonRow[];
    readonly reads: {
      readonly yes: string;
      readonly partial: string;
      readonly no: string;
    };
  };

  readonly ladder: SectionHead & {
    readonly rungs: readonly RungCopy[];
  };

  readonly downloads: SectionHead & {
    readonly note: string;
    /** Short technical facts under the grid; kept from the cut Privacy block. */
    readonly specs: readonly string[];
    readonly getLabel: string;
    readonly serverSubhead: string;
    readonly channels: Readonly<Record<ReleaseChannel, string>>;
    /** The player-facing grid. */
    readonly client: readonly PlatformCopy[];
    /** The addon and plugin grid under `serverSubhead`. */
    readonly server: readonly PlatformCopy[];
  };

  readonly operators: SectionHead & {
    readonly steps: readonly StepCopy[];
    readonly note: string;
    readonly actions: {
      readonly guide: LinkCopy;
      readonly bedrock: LinkCopy;
      readonly java: LinkCopy;
    };
  };

  readonly integrations: SectionHead & {
    readonly deck: {
      readonly label: string;
      readonly heading: string;
      readonly body: string;
      readonly cta: LinkCopy;
      readonly ariaLabel: string;
    };
    readonly apisLabel: string;
    /** Order matches APIS in lib/site.ts, which supplies spec and accent. */
    readonly apis: readonly ApiCopy[];
    readonly wikiCta: LinkCopy;
    readonly pendingDocs: string;
  };

  readonly faq: SectionHead & {
    readonly entries: readonly FaqEntry[];
  };

  readonly wiki: SectionHead & {
    readonly entries: readonly WikiEntry[];
    readonly browse: LinkCopy;
    readonly apiReference: LinkCopy;
  };

  readonly stickyCta: {
    readonly messages: Readonly<Record<Audience, string>> & {
      /** Used when FEATURES.demoServer is off, since the default promises a demo. */
      readonly playerNoDemo: string;
    };
    readonly demo: LinkCopy;
    readonly download: LinkCopy;
    readonly installGuide: LinkCopy;
  };

  readonly discordCallback: {
    readonly meta: MetaCopy;
    /** Label at the Discord end of the link bar. */
    readonly from: string;
    /**
     * Every outcome but `idle` reopens the app. Their `href` is only the
     * fallback the click handler replaces: it cannot carry the payload, because
     * by the time anyone clicks, the URL it came from has been stripped.
     */
    readonly outcomes: Readonly<Record<DiscordOutcome, OutcomeCopy>>;
    /** Precedes the identifier Discord sent back. */
    readonly codeLabel: string;
    readonly footnote: string;
    readonly home: LinkCopy;
  };

  readonly enrolled: {
    readonly meta: MetaCopy;
    readonly outcomes: Readonly<Record<EnrollOutcome, OutcomeCopy>>;
    /** Precedes the enrollment token itself. */
    readonly tokenLabel: string;
    /** Where the token goes, shown beneath it. */
    readonly instructions: string;
    readonly copyAction: string;
    readonly copied: string;
    /** Shown while the claim is being redeemed. */
    readonly redeeming: string;
    readonly footnote: string;
    readonly home: LinkCopy;
  };

  readonly footer: {
    readonly blurb: string;
    /** Rendered in order, each column in order. */
    readonly columns: readonly FooterColumn[];
    readonly copyright: (year: number) => string;
    readonly source: string;
  };
}
