<script lang="ts">
  import "../../css/app.css";
  import App from "../../js/app/app.ts";
  import type { Severity } from "$radial/core/controllers/Diagnostics";
  import type { IconName } from "$radial/core/icons/Icons";
  import RadFrame from "../../components/shell/RadFrame.svelte";
  import FaultScreen from "../../components/error/FaultScreen.svelte";
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { getVersion } from "@tauri-apps/api/app";
  import { stopForegroundService, isServiceRunning } from "tauri-plugin-audio-permissions";
  import { Store } from "@tauri-apps/plugin-store";
  import HelpLinks from "../../js/app/HelpLinks";
  import PlatformDetector from "../../js/app/utils/PlatformDetector";

  /**
   * Tears down audio streams, network streams, and the foreground service.
   * Called on mount so that any navigation to an error page results in a clean state.
   */
  async function teardown(): Promise<void> {
    try { await invoke("stop_audio_device", { device: "InputDevice" }); } catch (_) {}
    try { await invoke("stop_audio_device", { device: "OutputDevice" }); } catch (_) {}
    try { await invoke("stop_network_stream"); } catch (_) {}

    const platformDetector = new PlatformDetector();
    if (await platformDetector.checkMobile()) {
      try {
        const status = await isServiceRunning();
        if (status.running) {
          await stopForegroundService();
        }
      } catch (_) {}
    }
  }

  interface ErrorAction {
    label: string;
    url: string;
  }

  interface ErrorDefinition {
    code: string;
    title: string;
    message: string;
    /** The glyph at the centre of the break. */
    icon: IconName;
    /**
     * bad · something is broken. warn · a person has to act before this can work, and
     * nothing here is anyone's mistake. ok · not a failure at all.
     */
    severity: Severity;
    /** What this is about: the caption's label, and the eyebrow over the title. */
    category: string;
    /**
     * Replaces the eyebrow with a severity chip. Only the screens that are not reporting
     * a break have one, because leading with "nothing is wrong" is only worth doing when
     * it is true.
     */
    chip?: string;
    /** The state in two or three words. Joined to the code in the caption. */
    caption: string;
    /** Right of the top bar. */
    label: string;
    /** Left of the footbar: the usual cause, or a reassurance. */
    hint: string;
    primaryAction: ErrorAction;
    secondaryAction?: ErrorAction;
  }

  // Error configuration object - add new error codes here
  const ERROR_DEFINITIONS: Record<string, ErrorDefinition> = {
    'PERM1': {
      code: 'PERM1',
      title: 'Microphone Permission Required',
      message: 'Bedrock Voice Chat needs access to your microphone to enable voice communication. Please grant microphone permissions in your system settings and return to the dashboard.',
      icon: 'micoff',
      severity: 'bad',
      category: 'Audio input',
      caption: 'NOT PERMITTED',
      label: "Can't start",
      hint: 'Granted in your system settings, not here',
      primaryAction: {
        label: 'Back to Dashboard',
        url: '/dashboard'
      }
    },
    'PERM2': {
      code: 'PERM2',
      title: 'Notification Permission Required',
      message: 'Bedrock Voice Chat needs access to send you notifications on your platform to allow for background audio recording. Please grant notification permissions in your system settings and then return to the dashboard.',
      icon: 'belloff',
      severity: 'bad',
      category: 'Notifications',
      caption: 'NOT PERMITTED',
      label: "Can't start",
      hint: 'Background audio depends on this',
      primaryAction: {
        label: 'Back to Dashboard',
        url: '/dashboard'
      }
    },
    'AUDI01': {
      code: 'AUDI01',
      title: 'Incompatible Audio Input Device',
      message: 'Your microphone configuration is not supported. Make sure your input device on Windows is running at 48kHz sample rate, or change to a different input device',
      icon: 'wave',
      severity: 'bad',
      category: 'Audio input',
      caption: 'FORMAT UNSUPPORTED',
      label: "Can't start",
      hint: '48kHz is the sample rate to look for',
      primaryAction: {
        label: 'Change Audio Devices',
        url: '/settings'
      },
      secondaryAction: {
        label: 'View FAQ',
        url: 'https://github.com/Alaydriem/bedrock-voice-chat/discussions/32'
      }
    },
    'AUDI02': {
      code: 'AUDI02',
      title: 'No Input Device Found',
      message: 'No microphone was detected on your system. Connect a microphone or headset, verify it appears in your system sound settings, then try again.',
      icon: 'micoff',
      severity: 'bad',
      category: 'Audio input',
      caption: 'NO DEVICE',
      label: "Can't start",
      hint: 'A muted or unplugged headset is the usual cause',
      primaryAction: {
        label: 'Try Again',
        url: '/dashboard'
      },
      secondaryAction: {
        label: 'Change Audio Devices',
        url: '/settings'
      }
    },
    'AUDI03': {
      code: 'AUDI03',
      title: 'No Output Device Found',
      message: 'No speakers or headphones were detected on your system. Connect an audio output device, verify it appears in your system sound settings, then try again.',
      icon: 'speakeroff',
      severity: 'bad',
      category: 'Audio output',
      caption: 'NO DEVICE',
      label: "Can't start",
      hint: 'A muted or unplugged headset is the usual cause',
      primaryAction: {
        label: 'Try Again',
        url: '/dashboard'
      },
      secondaryAction: {
        label: 'Change Audio Devices',
        url: '/settings'
      }
    },
    'VER01': {
      code: 'VER01',
      title: 'Client Update Required',
      message: 'Your client version is outdated. Download the latest BVC client to connect to this server.',
      icon: 'download',
      severity: 'bad',
      category: 'Client',
      caption: 'TOO OLD',
      label: 'Incompatible',
      hint: 'The server is newer than this client',
      primaryAction: {
        label: 'Back to Dashboard',
        url: '/dashboard'
      }
    },
    'VER02': {
      code: 'VER02',
      title: 'Server Update Required',
      message: 'The server is running an older version of BVC that this client cannot connect to. Contact your server owner to have them update to the latest version.',
      icon: 'server',
      severity: 'bad',
      category: 'Server',
      caption: 'TOO OLD',
      label: 'Incompatible',
      hint: 'Only the server owner can fix this one',
      primaryAction: {
        label: 'Choose Different Server',
        url: '/server'
      }
    },
    'DNS01': {
      code: 'DNS01',
      title: 'DNS Resolution Failed',
      message: 'Could not resolve the server address. Check that the server URL is correct and that your internet connection is working. If you are using a custom DNS provider, ensure it can resolve the server hostname.',
      icon: 'globe',
      severity: 'bad',
      category: 'Address',
      caption: 'UNRESOLVED',
      label: 'Not connected',
      hint: 'A typo in the address is the usual cause',
      primaryAction: {
        label: 'Try Again',
        url: '/dashboard'
      },
      secondaryAction: {
        label: 'Choose Different Server',
        url: '/server'
      }
    },
    'QUIC01': {
      code: 'QUIC01',
      title: 'Voice Connection Blocked',
      message: 'HTTP connection succeeded but the voice connection (QUIC/UDP) was blocked. This usually means a firewall is blocking UDP traffic on the voice port. Check your firewall settings or contact your server administrator.',
      icon: 'shieldoff',
      severity: 'bad',
      category: 'Voice path',
      caption: 'UDP BLOCKED',
      label: 'Not connected',
      hint: 'HTTP got through; UDP did not',
      primaryAction: {
        label: 'Try Again',
        url: '/dashboard'
      },
      secondaryAction: {
        label: 'Choose Different Server',
        url: '/server'
      }
    },
    'CONN01': {
      code: 'CONN01',
      title: 'Connection Failed',
      message: 'Unable to establish a voice connection to the server. The server may be offline, unreachable, or your certificates may have expired. Please try again.',
      icon: 'unlink',
      severity: 'bad',
      category: 'Voice path',
      caption: 'NO ANSWER',
      label: 'Not connected',
      hint: 'A server that is not running answers exactly like this',
      primaryAction: {
        label: 'Try Again',
        url: '/dashboard'
      },
      secondaryAction: {
        label: 'Choose Different Server',
        url: '/server'
      }
    },
    'AUTH01': {
      code: 'AUTH01',
      title: 'Connection Refused',
      message: 'The server refused this connection because it could not verify your identity. Your certificate may have been revoked or reissued. Sign in again to get fresh credentials.',
      icon: 'cert',
      severity: 'bad',
      category: 'Identity',
      caption: 'NOT VERIFIED',
      label: 'Refused',
      hint: 'Signing in again issues fresh credentials',
      primaryAction: {
        label: 'Sign In Again',
        url: '/login'
      },
      secondaryAction: {
        label: 'Choose Different Server',
        url: '/server'
      }
    },
    'AUTH02': {
      code: 'AUTH02',
      title: 'Access Denied',
      message: 'This server has not granted you account access. Join the Minecraft server in-game at least once so it can register you, or ask the server operator to add you.',
      icon: 'lock',
      severity: 'warn',
      category: 'Account',
      chip: 'Not yet',
      caption: 'NOT REGISTERED',
      label: 'No access',
      hint: 'Nothing is wrong with your setup',
      primaryAction: {
        label: 'Try Again',
        url: '/login'
      },
      secondaryAction: {
        label: 'Choose Different Server',
        url: '/server'
      }
    },
    'UPD01': {
      code: 'UPD01',
      title: 'Update Available',
      message: 'A new version of Bedrock Voice Chat is available. This will download and install the update immediately.',
      icon: 'download',
      severity: 'ok',
      category: 'New version',
      chip: 'Available now',
      caption: 'READY',
      label: 'Ready to install',
      hint: 'Takes about a minute on a normal connection',
      primaryAction: {
        label: 'Update Now',
        url: ''
      },
      secondaryAction: {
        label: 'Stay on This Version',
        url: '/server'
      }
    },
    'AGE01': {
      code: 'AGE01',
      title: 'We need a grown-ups help to get started',
      message: "To use Bedrock Voice Chat, a parent or guardian needs to turn it on for your device. Ask them to help you unlock it -- then you're good to go!",
      icon: 'shield',
      severity: 'warn',
      category: 'Account',
      chip: 'Not yet',
      caption: 'NEEDS UNLOCKING',
      label: 'Locked',
      hint: 'Nothing is wrong with your setup',
      primaryAction: {
        label: 'Choose Different Server',
        url: '/server'
      },
      secondaryAction: {
        label: 'How do I unlock this?',
        url: 'https://github.com/Alaydriem/bedrock-voice-chat/wiki/Age-Signals-&-Declared-Age-Ranges'
      }
    },
    // Default error (used when code is not found or not provided)
    'DEFAULT': {
      code: 'ERROR',
      title: 'Something Went Wrong',
      message: 'An unexpected error occurred. Please try again or contact support if the problem persists.',
      icon: 'warn',
      severity: 'bad',
      category: 'Unknown',
      caption: 'UNEXPECTED',
      label: "Can't continue",
      hint: 'The code above is what support will ask for',
      primaryAction: {
        label: 'Back to Home',
        url: '/dashboard'
      }
    }
  };

  let currentError = $state(ERROR_DEFINITIONS['DEFAULT']);
  let appVersion = $state("");

  const UPDATE_PHRASES: readonly string[] = [
    'Downloading the update…',
    'Verifying the signature…',
    'Almost there…',
  ];

  let isUpdating = $state(false);
  let updateError = $state<string | null>(null);

  async function handleUpdate(): Promise<void> {
    isUpdating = true;
    updateError = null;
    try {
      await invoke("install_update");
    } catch (e) {
      updateError = String(e);
      isUpdating = false;
    }
  }

  /**
   * An external link opens in the system browser; everything else is a route. The old
   * page used anchors and let the WebView work it out, which navigated the app window to
   * GitHub with no way back.
   */
  function go(url: string): void {
    if (url.startsWith("http")) {
      void openUrl(url);
      return;
    }
    window.location.href = url;
  }

  const actions = $derived.by(() => {
    const isUpdate = currentError.code === 'UPD01';
    const list = [
      {
        label: isUpdate && isUpdating ? 'Updating…' : currentError.primaryAction.label,
        onclick: isUpdate ? handleUpdate : () => go(currentError.primaryAction.url),
        primary: true,
        disabled: isUpdating,
      },
    ];
    if (currentError.secondaryAction) {
      list.push({
        label: currentError.secondaryAction.label,
        onclick: () => go(currentError.secondaryAction!.url),
        primary: false,
        disabled: isUpdating,
      });
    }
    return list;
  });

  onMount(async () => {
    window.App = new App();
    window.dispatchEvent(new CustomEvent("app:mounted"));
    window.App.preloader();

    try { appVersion = await getVersion(); } catch (_) {}

    // Read error code from query parameter
    const urlParams = new URLSearchParams(window.location.search);
    const errorCode = urlParams.get('code');

    // Skip teardown for update page — no streams running from splash
    if (errorCode !== 'UPD01') {
      await teardown();
    }

    if (errorCode && ERROR_DEFINITIONS[errorCode]) {
      currentError = ERROR_DEFINITIONS[errorCode];
    } else if (errorCode) {
      currentError = {
        ...ERROR_DEFINITIONS['DEFAULT'],
        code: errorCode
      };
    }

    // Append version to update message if provided
    const version = urlParams.get('version');
    if (errorCode === 'UPD01' && version) {
      currentError = {
        ...currentError,
        message: `A new version (v${version}) of Bedrock Voice Chat is available. This will download and install the update immediately.`,
        caption: `v${version}`
      };
    }

    // Only show "Choose Different Server" actions when multiple servers exist
    // Skip for UPD01 — "Stay on This Version" should always be available
    if (errorCode !== 'UPD01') {
      const store = await Store.load("store.json", { autoSave: false, defaults: {} });
      const serverList = await store.get("server_list") as Array<{ server: string; player: string }> | null;
      const hasMultipleServers = serverList != null && serverList.length > 1;

      if (!hasMultipleServers) {
        if (currentError.secondaryAction?.url === '/server') {
          currentError = { ...currentError, secondaryAction: undefined };
        }
        if (currentError.primaryAction.url === '/server') {
          currentError = {
            ...currentError,
            primaryAction: { ...ERROR_DEFINITIONS['DEFAULT'].primaryAction }
          };
        }
      }
    }
  });
</script>

<RadFrame>
  <FaultScreen
    code={currentError.code}
    title={currentError.title}
    message={currentError.message}
    icon={currentError.icon}
    severity={currentError.severity}
    category={currentError.category}
    chip={currentError.chip}
    caption={currentError.caption}
    label={currentError.label}
    hint={currentError.hint}
    {appVersion}
    {actions}
    working={isUpdating}
    workingPhrases={UPDATE_PHRASES}
  >
    {#snippet footnote()}
      {#if updateError}
        <div class="rad-callout rad-callout--bad rad-rise" style="--d: 340; margin-top: 18px">
          <span>The update could not be installed. Try again, or download the new version by hand.</span>
        </div>
      {:else if currentError.severity !== 'ok'}
        <span class="rad-label rad-rise" style="--d: 360; display: block; margin-top: 26px">
          Still stuck?
        </span>
        <div class="rad-swatchrow rad-rise" style="--d: 390">
          <button class="rad-pill-link" onclick={() => void HelpLinks.openWiki()}>
            Wiki <span class="rad-pill-link__ext">&#8599;</span>
          </button>
          <button class="rad-pill-link" onclick={() => void HelpLinks.openDiscord()}>
            Discord <span class="rad-pill-link__ext">&#8599;</span>
          </button>
        </div>
      {/if}
    {/snippet}
  </FaultScreen>
</RadFrame>
