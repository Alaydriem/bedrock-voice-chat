/**
 * Which files hold shippable interface copy.
 *
 * Shared by the extractor and the coverage report so the two cannot disagree about what
 * the denominator is: a path counted as work by one and skipped by the other would make
 * the coverage number quietly wrong.
 */
export default class Sources {
  static readonly GLOB = "src/**/*.{ts,svelte}";

  static readonly IGNORED: readonly string[] = [
    // The design reference gallery. Its copy duplicates the shipped components, so
    // extracting it would double the catalog for no user-visible benefit.
    "src/radial/examples/",
    // Both test suites: vitest's, and the radial kit's node --test files.
    "src/test/",
    "src/radial/tests/",
    // Generated from the Rust types; the generator is what is under test, not its output.
    "src/js/bindings/",
    // The localization modules themselves hold marker names, not copy.
    "src/lib/i18n/",
    // The framework-free kit. It cannot reach the translation surface — no bundler under
    // `node --test`, and the surface is built on runes the type stripper cannot evaluate —
    // so it holds declared English defaults and takes translated words as arguments. The
    // app-side sets, `DiagnosticsCopy.labels()` among them, are what get counted.
    "src/radial/core/",
    "src/radial/bindings/",
    // Paints before the application boots, and is built by its own vite config with no
    // `$lib` alias. There is no pack loaded when this renders.
    "src/preloader/",
  ];

  static isIgnored(path: string): boolean {
    const normalized = path.split("\\").join("/");
    return Sources.IGNORED.some((prefix) => normalized.includes(prefix));
  }

  /**
   * Names that stay English wherever they appear.
   *
   * Translating a product, a platform or a protocol produces text that matches nothing the
   * reader can find — not in the game, not on their device, not in a search.
   */
  static readonly PROPER: ReadonlySet<string> = new Set([
    "Bedrock Voice Chat", "Minecraft Realms", "Minecraft Bedrock", "Minecraft",
    "Discord", "Stream Deck", "Xbox", "PlayStation", "Switch", "Windows", "macOS",
    "Linux", "Android", "iOS", "Aternos", "Geyser", "Floodgate", "Fabric", "Paper",
    "Java", "BVC", "WAV", "MP4", "QUIC", "UDP", "HTTP", "HTTPS", "DNS", "ASIO",
    "Opus", "WebSocket", "TLS", "mTLS", "VPS",
  ]);

  /**
   * True when every token is a proper noun — `Windows · macOS · Linux`,
   * `Java + Geyser & Floodgate`, `TLS 1.3 · mTLS`. Version numbers count as part of a name.
   */
  /**
   * Key names as they are printed on the key. `Num Enter` is what the keycap says, in every
   * locale that ships that keyboard, so translating it names a key nobody has.
   */
  static readonly KEY_NAME = /^(?:Num|Numpad|Fn|Alt|Ctrl|Shift|Meta|Cmd|Win)\b[\s\S]{0,12}$/;

  static isProperNoun(text: string): boolean {
    const trimmed = text.trim().replace(/\s+/g, " ");
    if (Sources.PROPER.has(trimmed)) return true;
    if (Sources.KEY_NAME.test(trimmed)) return true;

    const tokens = trimmed.split(/[\s·+&,—–]+/).filter(Boolean);
    return (
      tokens.length > 0 &&
      tokens.every((token) => Sources.PROPER.has(token) || /^v?\d+(\.\d+)*$/.test(token))
    );
  }

  /**
   * Log calls. Their arguments reach a file, never a reader, and marking them would put
   * `DiscordCallbackHandler: processing callback` in front of a translator.
   */
  static readonly LOG_CALL = /\b(?:warn|info|debug|error|trace|log)\s*\(\s*$/;
}
