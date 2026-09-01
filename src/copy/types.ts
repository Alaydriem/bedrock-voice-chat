import type { Audience, DiscordOutcome, EnrollOutcome } from '../lib/site';

/**
 * The contract every locale has to satisfy.
 *
 * Words live here; URLs, colours, anchors and platform data stay in lib/site.ts.
 * That split is what makes translation tractable: a translator touches exactly
 * one file per language and never has to understand a link target or a spectrum
 * token, and a mistranslated URL is impossible because there are no URLs to
 * mistranslate.
 *
 * Locales are typed against this interface, so a missing or misspelled key is a
 * build failure rather than an `undefined` rendered into the page. Adding a
 * string to the site means adding it here first, and every language then fails
 * to compile until it is filled in. That is deliberate: silent English fallback
 * is how half-translated sites happen.
 */

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
  readonly cta: string;
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
  readonly cta: string;
}

export interface StepCopy {
  readonly title: string;
  readonly detail: string;
  readonly time: string;
}

export interface FaqEntry {
  readonly q: string;
  readonly a: string;
  readonly linkLabel?: string;
}

export interface WikiEntry {
  readonly who: string;
  readonly title: string;
  readonly blurb: string;
}

export interface ApiCopy {
  readonly name: string;
  readonly what: string;
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
  readonly action: string;
  /** Left out where one action is the whole answer. */
  readonly secondary?: string;
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
    /** Keyed by the entry ids in lib/site.ts. */
    readonly links: Readonly<Record<string, string>>;
    readonly discord: string;
    readonly download: string;
  };

  readonly hero: {
    readonly eyebrow: string;
    readonly tracks: Readonly<Record<Audience, HeroTrack>>;
    readonly demoCta: string;
    readonly ringLabel: string;
    readonly creatorAside: string;
  };

  readonly switcher: {
    readonly label: string;
    readonly ariaLabel: string;
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
    readonly channels: {
      readonly stable: string;
      readonly beta: string;
      readonly testflight: string;
    };
  };

  readonly operators: SectionHead & {
    readonly steps: readonly StepCopy[];
    readonly note: string;
    readonly actions: {
      readonly guide: string;
      readonly bedrock: string;
      readonly java: string;
    };
  };

  readonly integrations: SectionHead & {
    readonly deck: {
      readonly label: string;
      readonly heading: string;
      readonly body: string;
      readonly cta: string;
      readonly ariaLabel: string;
    };
    readonly apisLabel: string;
    readonly apis: readonly ApiCopy[];
    readonly wikiCta: string;
    readonly pendingDocs: string;
  };

  readonly faq: SectionHead & {
    readonly entries: readonly FaqEntry[];
  };

  readonly wiki: SectionHead & {
    readonly entries: readonly WikiEntry[];
    readonly browse: string;
    readonly apiReference: string;
  };

  readonly stickyCta: {
    readonly messages: Readonly<Record<Audience, string>> & {
      /** Used when FEATURES.demoServer is off, since the default promises a demo. */
      readonly playerNoDemo: string;
    };
    readonly demo: string;
    readonly download: string;
    readonly installGuide: string;
  };

  readonly discordCallback: {
    readonly meta: MetaCopy;
    /** Label at the Discord end of the link bar. */
    readonly from: string;
    readonly outcomes: Readonly<Record<DiscordOutcome, OutcomeCopy>>;
    /** Precedes the identifier Discord sent back. */
    readonly codeLabel: string;
    readonly footnote: string;
    readonly home: string;
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
    readonly home: string;
  };

  readonly footer: {
    readonly blurb: string;
    readonly columns: Readonly<Record<string, string>>;
    readonly links: Readonly<Record<string, string>>;
    readonly copyright: (year: number) => string;
    readonly source: string;
  };
}
