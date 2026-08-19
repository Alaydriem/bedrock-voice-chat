import { I18n } from "$lib/i18n";
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
  static get UPDATE_PHRASES(): readonly string[] {
    return [
      I18n.t("Downloading the update…"),
      I18n.t("Verifying the signature…"),
      I18n.t("Almost there…"),
    ];
  }

  /**
   * A getter, not a field.
   *
   * As an object literal this is evaluated once when the module is first imported, which
   * is before any language pack has loaded — so every string would be captured in English
   * and never change again. Nothing about that fails loudly: the screen reads correctly
   * and is simply never translated.
   */
  static get DEFINITIONS(): Readonly<Record<string, FaultDefinition>> {
    return {
      PERM1: {
        code: "PERM1",
        title: I18n.t("Microphone Permission Required"),
        message: I18n.t(
          "Bedrock Voice Chat needs access to your microphone to enable voice communication. Please grant microphone permissions in your system settings and return to the dashboard.",
        ),
        icon: "micoff",
        severity: "bad",
        category: I18n.t("Audio input"),
        caption: I18n.t("NOT PERMITTED"),
        label: I18n.t("Can't start"),
        hint: I18n.t("Enable microphone permissions in system settings"),
        primaryAction: { label: I18n.t("Back to Dashboard"), url: "/dashboard" },
      },
      PERM2: {
        code: "PERM2",
        title: I18n.t("Notification Permission Required"),
        message: I18n.t(
          "Bedrock Voice Chat needs access to send you notifications on your platform to allow for background audio recording. Please grant notification permissions in your system settings and then return to the dashboard.",
        ),
        icon: "belloff",
        severity: "bad",
        category: I18n.t("Notifications"),
        caption: I18n.t("NOT PERMITTED"),
        label: I18n.t("Can't start"),
        hint: I18n.t("Enable notification permissions in system settings"),
        primaryAction: { label: I18n.t("Back to Dashboard"), url: "/dashboard" },
      },
      SERV01: {
        code: "SERV01",
        title: I18n.t("Background Audio Couldn't Start"),
        message: I18n.t(
          "Bedrock Voice Chat could not start the service that keeps your microphone live while the app is not in front. Check that your system is not restricting the app in the background, then try again.",
        ),
        icon: "rec",
        severity: "bad",
        category: I18n.t("Background audio"),
        caption: I18n.t("SERVICE STOPPED"),
        label: I18n.t("Can't start"),
        hint: I18n.t("Enable notification permissions in system settings"),
        primaryAction: { label: I18n.t("Try Again"), url: "/dashboard" },
      },
      AUDI01: {
        code: "AUDI01",
        title: I18n.t("Incompatible Audio Input Device"),
        message: I18n.t(
          "Your microphone configuration is not supported. Make sure your input device on Windows is running at 48kHz sample rate, or change to a different input device",
        ),
        icon: "wave",
        severity: "bad",
        category: I18n.t("Audio input"),
        caption: I18n.t("FORMAT UNSUPPORTED"),
        label: I18n.t("Can't start"),
        hint: I18n.t("Change your device to 48kHz"),
        primaryAction: { label: I18n.t("Change Audio Devices"), url: "/settings#audio" },
        secondaryAction: {
          label: I18n.t("View FAQ"),
          url: "https://www.bedrockvoicechat.com/wiki/player/settings/",
        },
      },
      AUDI02: {
        code: "AUDI02",
        title: I18n.t("No Input Device Found"),
        message: I18n.t(
          "No microphone was detected on your system. Connect a microphone or headset, verify it appears in your system sound settings, then try again.",
        ),
        icon: "micoff",
        severity: "bad",
        category: I18n.t("Audio input"),
        caption: I18n.t("NO DEVICE"),
        label: I18n.t("Can't start"),
        hint: I18n.t("Make sure your headphones or DAC is connected"),
        primaryAction: { label: I18n.t("Try Again"), url: "/dashboard" },
        secondaryAction: { label: I18n.t("Change Audio Devices"), url: "/settings#audio" },
      },
      AUDI03: {
        code: "AUDI03",
        title: I18n.t("No Output Device Found"),
        message: I18n.t(
          "No speakers or headphones were detected on your system. Connect an audio output device, verify it appears in your system sound settings, then try again.",
        ),
        icon: "speakeroff",
        severity: "bad",
        category: I18n.t("Audio output"),
        caption: I18n.t("NO DEVICE"),
        label: I18n.t("Can't start"),
        hint: I18n.t("Make sure your headphones or DAC is connected"),
        primaryAction: { label: I18n.t("Try Again"), url: "/dashboard" },
        secondaryAction: { label: I18n.t("Change Audio Devices"), url: "/settings#audio" },
      },
      VER01: {
        code: "VER01",
        title: I18n.t("Client Update Required"),
        message: I18n.t(
          "Your client version is outdated. Download the latest BVC client to connect to this server.",
        ),
        icon: "download",
        severity: "bad",
        category: I18n.t("Client"),
        caption: I18n.tc("client", "TOO OLD"),
        label: I18n.t("Incompatible"),
        hint: I18n.t("Update Bedrock Voice Chat to the latest version"),
        primaryAction: { label: I18n.t("Back to Dashboard"), url: "/dashboard" },
      },
      VER02: {
        code: "VER02",
        title: I18n.t("Server Update Required"),
        message: I18n.t(
          "The server is running an older version of BVC that this client cannot connect to. Contact your server owner to have them update to the latest version.",
        ),
        icon: "server",
        severity: "bad",
        category: I18n.t("Server"),
        caption: I18n.tc("server", "TOO OLD"),
        label: I18n.t("Incompatible"),
        hint: I18n.t("Your server operator needs to update BVC"),
        primaryAction: { label: I18n.t("Choose Different Server"), url: "/" },
      },
      DNS01: {
        code: "DNS01",
        title: I18n.t("DNS Resolution Failed"),
        message: I18n.t(
          "Could not resolve the server address. Check that the server URL is correct and that your internet connection is working. If you are using a custom DNS provider, ensure it can resolve the server hostname.",
        ),
        icon: "globe",
        severity: "bad",
        category: I18n.t("Address"),
        caption: I18n.t("UNRESOLVED"),
        label: I18n.t("Not connected"),
        hint: I18n.t("Check the name, and that your server is online and running."),
        primaryAction: { label: I18n.t("Try Again"), url: "/dashboard" },
        secondaryAction: { label: I18n.t("Choose Different Server"), url: "/" },
      },
      QUIC01: {
        code: "QUIC01",
        title: I18n.t("Voice Connection Blocked"),
        message: I18n.t(
          "HTTP connection succeeded but the voice connection (QUIC/UDP) was blocked. This usually means a firewall is blocking UDP traffic on the voice port. Check your firewall settings or contact your server administrator.",
        ),
        icon: "shieldoff",
        severity: "bad",
        category: I18n.t("Voice path"),
        caption: I18n.t("UDP BLOCKED"),
        label: I18n.t("Not connected"),
        hint: I18n.t("Voice path is blocked"),
        primaryAction: { label: I18n.t("Try Again"), url: "/dashboard" },
        secondaryAction: { label: I18n.t("Choose Different Server"), url: "/" },
      },
      CONN01: {
        code: "CONN01",
        title: I18n.t("Connection Failed"),
        message: I18n.t(
          "Unable to establish a voice connection to the server. The server may be offline, unreachable, or your certificates may have expired. Please try again.",
        ),
        icon: "unlink",
        severity: "bad",
        category: I18n.t("Voice path"),
        caption: I18n.t("NO ANSWER"),
        label: I18n.t("Not connected"),
        hint: I18n.t("The server isn't responding"),
        primaryAction: { label: I18n.t("Try Again"), url: "/dashboard" },
        secondaryAction: { label: I18n.t("Choose Different Server"), url: "/" },
      },
      AUTH01: {
        code: "AUTH01",
        title: I18n.t("Connection Refused"),
        message: I18n.t(
          "The server refused this connection because it could not verify your identity. Your certificate may have been revoked or reissued. Sign in again to get fresh credentials.",
        ),
        icon: "cert",
        severity: "bad",
        category: I18n.t("Identity"),
        caption: I18n.t("NOT VERIFIED"),
        label: I18n.t("Refused"),
        hint: I18n.t("Try signing out then back in"),
        primaryAction: { label: I18n.t("Sign In Again"), url: "/login" },
        secondaryAction: { label: I18n.t("Choose Different Server"), url: "/" },
      },
      AUTH02: {
        code: "AUTH02",
        title: I18n.t("Access Denied"),
        message: I18n.t(
          "This server has not granted you account access. Join the Minecraft server in-game at least once so it can register you, or ask the server operator to add you.",
        ),
        icon: "lock",
        severity: "warn",
        category: I18n.t("Account"),
        chip: I18n.t("Not yet"),
        caption: I18n.t("NOT REGISTERED"),
        label: I18n.t("No access"),
        hint: I18n.t("You don't have permission to access this server"),
        primaryAction: { label: I18n.t("Try Again"), url: "/login" },
        secondaryAction: { label: I18n.t("Choose Different Server"), url: "/" },
      },
      AUTH03: {
        code: "AUTH03",
        title: I18n.t("Credentials Could Not Be Saved"),
        message: I18n.t(
          "You signed in, but this device could not save your credentials to its secure storage, so the sign-in could not be completed. This is usually temporary. Try again, and if it keeps happening, restart your device.",
        ),
        icon: "shieldoff",
        severity: "bad",
        category: I18n.t("Secure storage"),
        caption: I18n.t("NOT SAVED"),
        label: I18n.t("Can't save"),
        hint: I18n.t("Your device's secure storage rejected the write"),
        primaryAction: { label: I18n.t("Try Again"), url: "/login" },
        secondaryAction: { label: I18n.t("Choose Different Server"), url: "/" },
      },
      AUTH04: {
        code: "AUTH04",
        title: I18n.t("No Keyring Available"),
        message: I18n.t(
          "You signed in, but this device has no unlocked keyring to store your credentials in, so the sign-in could not be completed. On Linux, open Passwords and Keys, create a password keyring named Login, and set it as the default. If your system signs you in automatically, turning that off and signing in with your password will create the keyring for you.",
        ),
        icon: "lock",
        severity: "warn",
        category: I18n.t("Secure storage"),
        chip: I18n.t("Action needed"),
        caption: I18n.t("NO KEYRING"),
        label: I18n.t("Can't save"),
        hint: I18n.t("Create a Login keyring, then sign in again"),
        primaryAction: { label: I18n.t("Try Again"), url: "/login" },
        secondaryAction: { label: I18n.t("Choose Different Server"), url: "/" },
      },
      CERT01: {
        code: "CERT01",
        title: I18n.t("Server Certificate Rejected"),
        message: I18n.t(
          "This server's security certificate does not match the one this device was issued. Your saved sign-in for it has been removed. Sign in again to get fresh credentials. If it keeps happening, the server operator needs to fix their certificate configuration.",
        ),
        icon: "cert",
        severity: "bad",
        category: I18n.t("Identity"),
        caption: I18n.t("UNTRUSTED"),
        label: I18n.t("Signed out"),
        hint: I18n.t("You don't have permission to access this server."),
        primaryAction: { label: I18n.t("Sign In Again"), url: "/login" },
        secondaryAction: { label: I18n.t("Choose Different Server"), url: "/" },
      },
      CERT02: {
        code: "CERT02",
        title: I18n.t("Server Certificate Misconfigured"),
        message: I18n.t(
          "Your sign-in for this server is still valid, but its voice connection presented a certificate this device cannot verify. Only the server operator can fix this. Your saved sign-in has been kept.",
        ),
        icon: "cert",
        severity: "bad",
        category: I18n.t("Server"),
        caption: I18n.t("UNTRUSTED"),
        label: I18n.t("Not connected"),
        hint: I18n.t("Contact your server operator."),
        primaryAction: { label: I18n.t("Try Again"), url: "/dashboard" },
        secondaryAction: { label: I18n.t("Choose Different Server"), url: "/" },
      },
      UPD01: {
        code: "UPD01",
        title: I18n.t("Update Available"),
        message: I18n.t(
          "A new version of Bedrock Voice Chat is available. This will download and install the update immediately.",
        ),
        icon: "download",
        severity: "ok",
        category: I18n.t("New version"),
        chip: I18n.t("Available now"),
        caption: I18n.t("READY"),
        label: I18n.t("Ready to install"),
        hint: I18n.t("Takes about a minute on a normal connection"),
        primaryAction: { label: I18n.t("Update Now"), url: "" },
        secondaryAction: { label: I18n.t("Stay on This Version"), url: "/" },
      },
      AGE01: {
        code: "AGE01",
        title: I18n.t("We need a grown-ups help to get started"),
        message: I18n.t(
          "To use Bedrock Voice Chat, a parent or guardian needs to turn it on for your device. Ask them to help you unlock it -- then you're good to go!",
        ),
        icon: "shield",
        severity: "warn",
        category: I18n.t("Account"),
        chip: I18n.t("Not yet"),
        caption: I18n.t("NEEDS UNLOCKING"),
        label: I18n.t("Locked"),
        hint: I18n.t("Nothing is wrong with your setup"),
        primaryAction: { label: I18n.t("Choose Different Server"), url: "/" },
        secondaryAction: {
          label: I18n.t("How do I unlock this?"),
          url: "https://www.bedrockvoicechat.com/wiki/player/parents-and-age/",
        },
      },
      DEFAULT: {
        code: "ERROR",
        title: I18n.t("Something Went Wrong"),
        message: I18n.t(
          "An unexpected error occurred. Please try again or contact support if the problem persists.",
        ),
        icon: "warn",
        severity: "bad",
        category: I18n.t("Unknown"),
        caption: I18n.t("UNEXPECTED"),
        label: I18n.t("Can't continue"),
        hint: I18n.t("Take a screenshot of this page and report a issue in Discord"),
        primaryAction: { label: I18n.t("Back to Home"), url: "/dashboard" },
      },
    };
  }

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
    const definitions = FaultCatalog.DEFINITIONS;
    const fallback = definitions.DEFAULT;
    if (!code) return fallback;
    return definitions[code] ?? { ...fallback, code };
  }

  /** Names the version in the update's copy and caption, when the caller knows it. */
  static withVersion(definition: FaultDefinition, version: string | null): FaultDefinition {
    if (!version || definition.code !== FaultCatalog.UPDATE) return definition;
    return {
      ...definition,
      message: I18n.tf(
        "A new version (v{version}) of Bedrock Voice Chat is available. This will download and install the update immediately.",
        { version },
      ),
      caption: I18n.tf("v{version}", { version }),
    };
  }

  /**
   * The definition a screen should show, from the three things that decide it.
   *
   * The composition lives here rather than in the route so it can be tested without
   * mounting a page, and so the route holds only the inputs. Holding the *result* is what
   * froze this screen's language before: the strings are read on access, so a stored
   * definition keeps whatever was current when it was built.
   */
  static forScreen(
    code: string | null,
    version: string | null,
    singleServer: boolean,
  ): FaultDefinition {
    const base = FaultCatalog.withVersion(FaultCatalog.resolve(code), version);

    // "Stay on This Version" is not a server switch, so the update keeps both of its
    // actions however many servers are configured.
    if (!singleServer || FaultCatalog.isUpdate(code)) return base;
    return FaultCatalog.withoutServerSwitch(base);
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
