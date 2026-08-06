import type FaultDefinition from "./FaultDefinition";

/**
 * Every terminal state the app can land on, and the rules for adapting one to the machine
 * it is being shown on.
 *
 * Kept out of the route because the route is not the only thing that needs to read them:
 * the review harness renders all of them, and the checks that a definition is coherent —
 * that its icon exists, that a chip and a severity agree — cannot be written against copy
 * embedded in a page.
 */
export default class FaultCatalog {
  /** The one code that is not a failure, and the only screen that installs something. */
  static readonly UPDATE = "UPD01";

  /**
   * Shown under the mark while the update runs. Named states rather than a percentage: the
   * updater reports no progress, and a bar that does not move is worse than no bar.
   */
  static readonly UPDATE_PHRASES: readonly string[] = [
    "Downloading the update…",
    "Verifying the signature…",
    "Almost there…",
  ];

  static readonly DEFINITIONS: Readonly<Record<string, FaultDefinition>> = {
    PERM1: {
      code: "PERM1",
      title: "Microphone Permission Required",
      message:
        "Bedrock Voice Chat needs access to your microphone to enable voice communication. Please grant microphone permissions in your system settings and return to the dashboard.",
      icon: "micoff",
      severity: "bad",
      category: "Audio input",
      caption: "NOT PERMITTED",
      label: "Can't start",
      hint: "Granted in your system settings, not here",
      primaryAction: { label: "Back to Dashboard", url: "/dashboard" },
    },
    PERM2: {
      code: "PERM2",
      title: "Notification Permission Required",
      message:
        "Bedrock Voice Chat needs access to send you notifications on your platform to allow for background audio recording. Please grant notification permissions in your system settings and then return to the dashboard.",
      icon: "belloff",
      severity: "bad",
      category: "Notifications",
      caption: "NOT PERMITTED",
      label: "Can't start",
      hint: "Background audio depends on this",
      primaryAction: { label: "Back to Dashboard", url: "/dashboard" },
    },
    SERV01: {
      code: "SERV01",
      title: "Background Audio Couldn't Start",
      message:
        "Bedrock Voice Chat could not start the service that keeps your microphone live while the app is not in front. Check that your system is not restricting the app in the background, then try again.",
      icon: "rec",
      severity: "bad",
      category: "Background audio",
      caption: "SERVICE STOPPED",
      label: "Can't start",
      hint: "Battery optimisation is the usual cause",
      primaryAction: { label: "Try Again", url: "/dashboard" },
    },
    AUDI01: {
      code: "AUDI01",
      title: "Incompatible Audio Input Device",
      message:
        "Your microphone configuration is not supported. Make sure your input device on Windows is running at 48kHz sample rate, or change to a different input device",
      icon: "wave",
      severity: "bad",
      category: "Audio input",
      caption: "FORMAT UNSUPPORTED",
      label: "Can't start",
      hint: "48kHz is the sample rate to look for",
      primaryAction: { label: "Change Audio Devices", url: "/settings#audio" },
      secondaryAction: {
        label: "View FAQ",
        url: "https://github.com/Alaydriem/bedrock-voice-chat/discussions/32",
      },
    },
    AUDI02: {
      code: "AUDI02",
      title: "No Input Device Found",
      message:
        "No microphone was detected on your system. Connect a microphone or headset, verify it appears in your system sound settings, then try again.",
      icon: "micoff",
      severity: "bad",
      category: "Audio input",
      caption: "NO DEVICE",
      label: "Can't start",
      hint: "A muted or unplugged headset is the usual cause",
      primaryAction: { label: "Try Again", url: "/dashboard" },
      secondaryAction: { label: "Change Audio Devices", url: "/settings#audio" },
    },
    AUDI03: {
      code: "AUDI03",
      title: "No Output Device Found",
      message:
        "No speakers or headphones were detected on your system. Connect an audio output device, verify it appears in your system sound settings, then try again.",
      icon: "speakeroff",
      severity: "bad",
      category: "Audio output",
      caption: "NO DEVICE",
      label: "Can't start",
      hint: "A muted or unplugged headset is the usual cause",
      primaryAction: { label: "Try Again", url: "/dashboard" },
      secondaryAction: { label: "Change Audio Devices", url: "/settings#audio" },
    },
    VER01: {
      code: "VER01",
      title: "Client Update Required",
      message:
        "Your client version is outdated. Download the latest BVC client to connect to this server.",
      icon: "download",
      severity: "bad",
      category: "Client",
      caption: "TOO OLD",
      label: "Incompatible",
      hint: "The server is newer than this client",
      primaryAction: { label: "Back to Dashboard", url: "/dashboard" },
    },
    VER02: {
      code: "VER02",
      title: "Server Update Required",
      message:
        "The server is running an older version of BVC that this client cannot connect to. Contact your server owner to have them update to the latest version.",
      icon: "server",
      severity: "bad",
      category: "Server",
      caption: "TOO OLD",
      label: "Incompatible",
      hint: "Only the server owner can fix this one",
      primaryAction: { label: "Choose Different Server", url: "/" },
    },
    DNS01: {
      code: "DNS01",
      title: "DNS Resolution Failed",
      message:
        "Could not resolve the server address. Check that the server URL is correct and that your internet connection is working. If you are using a custom DNS provider, ensure it can resolve the server hostname.",
      icon: "globe",
      severity: "bad",
      category: "Address",
      caption: "UNRESOLVED",
      label: "Not connected",
      hint: "A typo in the address is the usual cause",
      primaryAction: { label: "Try Again", url: "/dashboard" },
      secondaryAction: { label: "Choose Different Server", url: "/" },
    },
    QUIC01: {
      code: "QUIC01",
      title: "Voice Connection Blocked",
      message:
        "HTTP connection succeeded but the voice connection (QUIC/UDP) was blocked. This usually means a firewall is blocking UDP traffic on the voice port. Check your firewall settings or contact your server administrator.",
      icon: "shieldoff",
      severity: "bad",
      category: "Voice path",
      caption: "UDP BLOCKED",
      label: "Not connected",
      hint: "HTTP got through; UDP did not",
      primaryAction: { label: "Try Again", url: "/dashboard" },
      secondaryAction: { label: "Choose Different Server", url: "/" },
    },
    CONN01: {
      code: "CONN01",
      title: "Connection Failed",
      message:
        "Unable to establish a voice connection to the server. The server may be offline, unreachable, or your certificates may have expired. Please try again.",
      icon: "unlink",
      severity: "bad",
      category: "Voice path",
      caption: "NO ANSWER",
      label: "Not connected",
      hint: "A server that is not running answers exactly like this",
      primaryAction: { label: "Try Again", url: "/dashboard" },
      secondaryAction: { label: "Choose Different Server", url: "/" },
    },
    AUTH01: {
      code: "AUTH01",
      title: "Connection Refused",
      message:
        "The server refused this connection because it could not verify your identity. Your certificate may have been revoked or reissued. Sign in again to get fresh credentials.",
      icon: "cert",
      severity: "bad",
      category: "Identity",
      caption: "NOT VERIFIED",
      label: "Refused",
      hint: "Signing in again issues fresh credentials",
      primaryAction: { label: "Sign In Again", url: "/login" },
      secondaryAction: { label: "Choose Different Server", url: "/" },
    },
    AUTH02: {
      code: "AUTH02",
      title: "Access Denied",
      message:
        "This server has not granted you account access. Join the Minecraft server in-game at least once so it can register you, or ask the server operator to add you.",
      icon: "lock",
      severity: "warn",
      category: "Account",
      chip: "Not yet",
      caption: "NOT REGISTERED",
      label: "No access",
      hint: "Nothing is wrong with your setup",
      primaryAction: { label: "Try Again", url: "/login" },
      secondaryAction: { label: "Choose Different Server", url: "/" },
    },
    UPD01: {
      code: "UPD01",
      title: "Update Available",
      message:
        "A new version of Bedrock Voice Chat is available. This will download and install the update immediately.",
      icon: "download",
      severity: "ok",
      category: "New version",
      chip: "Available now",
      caption: "READY",
      label: "Ready to install",
      hint: "Takes about a minute on a normal connection",
      primaryAction: { label: "Update Now", url: "" },
      secondaryAction: { label: "Stay on This Version", url: "/" },
    },
    AGE01: {
      code: "AGE01",
      title: "We need a grown-ups help to get started",
      message:
        "To use Bedrock Voice Chat, a parent or guardian needs to turn it on for your device. Ask them to help you unlock it -- then you're good to go!",
      icon: "shield",
      severity: "warn",
      category: "Account",
      chip: "Not yet",
      caption: "NEEDS UNLOCKING",
      label: "Locked",
      hint: "Nothing is wrong with your setup",
      primaryAction: { label: "Choose Different Server", url: "/" },
      secondaryAction: {
        label: "How do I unlock this?",
        url: "https://github.com/Alaydriem/bedrock-voice-chat/wiki/Age-Signals-&-Declared-Age-Ranges",
      },
    },
    DEFAULT: {
      code: "ERROR",
      title: "Something Went Wrong",
      message:
        "An unexpected error occurred. Please try again or contact support if the problem persists.",
      icon: "warn",
      severity: "bad",
      category: "Unknown",
      caption: "UNEXPECTED",
      label: "Can't continue",
      hint: "The code above is what support will ask for",
      primaryAction: { label: "Back to Home", url: "/dashboard" },
    },
  };

  static isUpdate(code: string | null): boolean {
    return code === FaultCatalog.UPDATE;
  }

  /**
   * The definition for a code, or the catch-all.
   *
   * An unrecognised code keeps its own code on the catch-all rather than being shown as
   * `ERROR`: whatever raised it is what support will be asked about, and a code the app
   * silently renamed cannot be looked up by anyone.
   */
  static resolve(code: string | null): FaultDefinition {
    const fallback = FaultCatalog.DEFINITIONS.DEFAULT;
    if (!code) return fallback;
    return FaultCatalog.DEFINITIONS[code] ?? { ...fallback, code };
  }

  /** Names the version in the update's copy and caption, when the caller knows it. */
  static withVersion(definition: FaultDefinition, version: string | null): FaultDefinition {
    if (!version || definition.code !== FaultCatalog.UPDATE) return definition;
    return {
      ...definition,
      message: `A new version (v${version}) of Bedrock Voice Chat is available. This will download and install the update immediately.`,
      caption: `v${version}`,
    };
  }

  /**
   * Removes the offer to switch servers.
   *
   * With one server configured there is nothing to switch to, and an action that leads to a
   * list of one is a dead end offered to someone who is already stuck. A primary action
   * that was the switch falls back to the catch-all's, so the screen is never left with no
   * way out at all.
   */
  static withoutServerSwitch(definition: FaultDefinition): FaultDefinition {
    const SWITCH = "/";
    let adjusted = definition;

    if (adjusted.secondaryAction?.url === SWITCH) {
      adjusted = { ...adjusted, secondaryAction: undefined };
    }
    if (adjusted.primaryAction.url === SWITCH) {
      adjusted = {
        ...adjusted,
        primaryAction: { ...FaultCatalog.DEFINITIONS.DEFAULT.primaryAction },
      };
    }

    return adjusted;
  }
}
