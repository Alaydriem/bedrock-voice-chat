<script lang="ts">
  import "../../css/app.css";
  import App from "../../js/app/app.ts";
  import { onMount } from 'svelte';
  import { invoke } from "@tauri-apps/api/core";
  import { stopForegroundService, isServiceRunning } from 'tauri-plugin-audio-permissions';
  import { Store } from "@tauri-apps/plugin-store";
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

  // Error configuration object - add new error codes here
  const ERROR_DEFINITIONS: Record<string, {
    code: string;
    title: string;
    message: string;
    icon?: string; // FontAwesome icon class
    primaryAction: {
      label: string;
      url: string;
      style?: 'primary' | 'secondary' | 'danger';
    };
    secondaryAction?: {
      label: string;
      url: string;
      style?: 'primary' | 'secondary' | 'danger';
    };
  }> = {
    'PERM1': {
      code: 'PERM1',
      title: 'Microphone Permission Required',
      message: 'Bedrock Voice Chat needs access to your microphone to enable voice communication. Please grant microphone permissions in your system settings and return to the dashboard.',
      icon: 'fa-solid fa-microphone-slash',
      primaryAction: {
        label: 'Back to Dashboard',
        url: '/dashboard',
        style: 'primary'
      }
    },
    'PERM2': {
      code: 'PERM2',
      title: 'Notification Permission Required',
      message: 'Bedrock Voice Chat needs access to send you notifications on your platform to allow for background audio recording. Please grant notification permissions in your system settings and then return to the dashboard.',
      icon: 'fa-solid fa-bell-slash',
      primaryAction: {
        label: 'Back to Dashboard',
        url: '/dashboard',
        style: 'primary'
      }
    },
    'AUDI01': {
      code: 'AUDI01',
      title: 'Incompatible Audio Input Device',
      message: 'Your microphone configuration is not supported. Make sure your input device on Windows is running at 48kHz sample rate, or change to a different input device',
      icon: 'fa-solid fa-volume-xmark',
      primaryAction: {
        label: 'Change Audio Devices',
        url: '/settings',
        style: 'primary'
      },
      secondaryAction: {
        label: 'View FAQ',
        url: 'https://github.com/Alaydriem/bedrock-voice-chat/discussions/32',
        style: 'secondary'
      }
    },
    'AUDI02': {
      code: 'AUDI02',
      title: 'No Input Device Found',
      message: 'No microphone was detected on your system. Connect a microphone or headset, verify it appears in your system sound settings, then try again.',
      icon: 'fa-solid fa-microphone-slash',
      primaryAction: {
        label: 'Try Again',
        url: '/dashboard',
        style: 'primary'
      },
      secondaryAction: {
        label: 'Change Audio Devices',
        url: '/settings',
        style: 'secondary'
      }
    },
    'AUDI03': {
      code: 'AUDI03',
      title: 'No Output Device Found',
      message: 'No speakers or headphones were detected on your system. Connect an audio output device, verify it appears in your system sound settings, then try again.',
      icon: 'fa-solid fa-volume-xmark',
      primaryAction: {
        label: 'Try Again',
        url: '/dashboard',
        style: 'primary'
      },
      secondaryAction: {
        label: 'Change Audio Devices',
        url: '/settings',
        style: 'secondary'
      }
    },
    'VER01': {
      code: 'VER01',
      title: 'Client Update Required',
      message: 'Your client version is outdated. Download the latest BVC client to connect to this server.',
      icon: 'fa-solid fa-cloud-arrow-down',
      primaryAction: {
        label: 'Back to Dashboard',
        url: '/dashboard',
        style: 'primary'
      }
    },
    'VER02': {
      code: 'VER02',
      title: 'Server Update Required',
      message: 'The server is running an older version of BVC that this client cannot connect to. Contact your server owner to have them update to the latest version.',
      icon: 'fa-solid fa-server',
      primaryAction: {
        label: 'Choose Different Server',
        url: '/server',
        style: 'primary'
      }
    },
    'DNS01': {
      code: 'DNS01',
      title: 'DNS Resolution Failed',
      message: 'Could not resolve the server address. Check that the server URL is correct and that your internet connection is working. If you are using a custom DNS provider, ensure it can resolve the server hostname.',
      icon: 'fa-solid fa-globe',
      primaryAction: {
        label: 'Try Again',
        url: '/dashboard',
        style: 'primary'
      },
      secondaryAction: {
        label: 'Choose Different Server',
        url: '/server',
        style: 'secondary'
      }
    },
    'QUIC01': {
      code: 'QUIC01',
      title: 'Voice Connection Blocked',
      message: 'HTTP connection succeeded but the voice connection (QUIC/UDP) was blocked. This usually means a firewall is blocking UDP traffic on the voice port. Check your firewall settings or contact your server administrator.',
      icon: 'fa-solid fa-shield-halved',
      primaryAction: {
        label: 'Try Again',
        url: '/dashboard',
        style: 'primary'
      },
      secondaryAction: {
        label: 'Choose Different Server',
        url: '/server',
        style: 'secondary'
      }
    },
    'CONN01': {
      code: 'CONN01',
      title: 'Connection Failed',
      message: 'Unable to establish a voice connection to the server. The server may be offline, unreachable, or your certificates may have expired. Please try again.',
      icon: 'fa-solid fa-plug-circle-xmark',
      primaryAction: {
        label: 'Try Again',
        url: '/dashboard',
        style: 'primary'
      },
      secondaryAction: {
        label: 'Choose Different Server',
        url: '/server',
        style: 'secondary'
      }
    },
    'UPD01': {
      code: 'UPD01',
      title: 'Update Available',
      message: 'A new version of Bedrock Voice Chat is available. This will download and install the update immediately.',
      icon: 'fa-solid fa-cloud-arrow-down',
      primaryAction: {
        label: 'Update Now',
        url: '',
        style: 'primary'
      },
      secondaryAction: {
        label: 'Stay on This Version',
        url: '/server',
        style: 'secondary'
      }
    },
    // Default error (used when code is not found or not provided)
    'DEFAULT': {
      code: 'ERROR',
      title: 'Something Went Wrong',
      message: 'An unexpected error occurred. Please try again or contact support if the problem persists.',
      icon: 'fa-solid fa-triangle-exclamation',
      primaryAction: {
        label: 'Back to Home',
        url: '/dashboard',
        style: 'primary'
      }
    }
  };

  let currentError = $state(ERROR_DEFINITIONS['DEFAULT']);

  // Button style configurations
  const BUTTON_STYLES = {
    primary: 'btn mt-5 w-full bg-primary font-medium text-white hover:bg-primary-focus focus:bg-primary-focus active:bg-primary-focus/90 dark:bg-accent dark:hover:bg-accent-focus dark:focus:bg-accent-focus dark:active:bg-accent/90',
    secondary: 'btn mt-3 w-full border border-slate-300 font-medium text-slate-800 hover:bg-slate-150 focus:bg-slate-150 active:bg-slate-150/80 dark:border-navy-450 dark:text-navy-50 dark:hover:bg-navy-500 dark:focus:bg-navy-500 dark:active:bg-navy-500/90',
    danger: 'btn mt-3 w-full bg-error font-medium text-white hover:bg-error-focus focus:bg-error-focus active:bg-error-focus/90'
  };

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

  onMount(async () => {
    window.App = new App();
    window.dispatchEvent(new CustomEvent("app:mounted"));
    window.App.preloader();

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
        message: `A new version (v${version}) of Bedrock Voice Chat is available. This will download and install the update immediately.`
      };
    }

    // Only show "Choose Different Server" actions when multiple servers exist
    // Skip for UPD01 — "Stay on This Version" should always be available
    if (errorCode !== 'UPD01') {
      const store = await Store.load("store.json", { autoSave: false });
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

<div
      id="root"
      class="min-h-80vh cloak flex grow bg-slate-50 dark:bg-navy-900"
    >
      <main class="grid w-full grow grid-cols-1 place-items-center">
        <div class="w-full max-w-[26rem] p-4 sm:px-5">
          <div class="text-center">
            <img
              class="mx-auto h-32 w-32"
              src="/images/app-logo-transparent.png"
              alt="Bedrock Voice Chat Logo"
            />
            <div class="mt-4">
              <h2
                class="text-2xl font-semibold text-slate-600 dark:text-navy-100"
              >
                Bedrock Voice Chat
              </h2>
            </div>
          </div>

          <div class="card mt-5 rounded-lg p-5 lg:p-7">
            <div class="text-center">
              <!-- Error Icon -->
              {#if currentError.icon}
                <div class="mb-4">
                  <i class="{currentError.icon} text-5xl {currentError.code === 'UPD01' ? 'text-success' : 'text-error'}"></i>
                </div>
              {/if}

              <!-- Error Code -->
              <p class="text-5xl font-bold {currentError.code === 'UPD01' ? 'text-success' : 'text-primary dark:text-accent'}">
                {currentError.code}
              </p>

              <!-- Error Title -->
              <p class="mt-4 text-xl font-semibold text-slate-800 dark:text-navy-50">
                {currentError.title}
              </p>

              <!-- Error Message -->
              <p class="mt-2 text-slate-500 dark:text-navy-200">
                {currentError.message}
              </p>

              {#if updateError}
                <div class="mt-3 rounded-lg bg-error/10 border border-error/20 p-3 text-sm text-error">
                  <i class="fa-solid fa-circle-exclamation mr-1"></i>
                  Update failed. Please try again.
                </div>
              {/if}

              <!-- Primary Action Button -->
              {#if currentError.code === 'UPD01'}
                <button
                  onclick={handleUpdate}
                  disabled={isUpdating}
                  class="{BUTTON_STYLES[currentError.primaryAction.style || 'primary']}{isUpdating ? ' opacity-75 cursor-not-allowed' : ''}"
                >
                  {#if isUpdating}
                    <i class="fa-solid fa-spinner fa-spin mr-2"></i>
                    Updating...
                  {:else}
                    {currentError.primaryAction.label}
                  {/if}
                </button>
              {:else}
                <a
                  href={currentError.primaryAction.url}
                  class={BUTTON_STYLES[currentError.primaryAction.style || 'primary']}
                >
                  {currentError.primaryAction.label}
                </a>
              {/if}

              <!-- Secondary Action Button (Optional) -->
              {#if currentError.secondaryAction}
                {#if isUpdating}
                  <span class="{BUTTON_STYLES[currentError.secondaryAction.style || 'secondary']} opacity-50 cursor-not-allowed pointer-events-none">
                    {currentError.secondaryAction.label}
                  </span>
                {:else if currentError.code === 'UPD01'}
                  <button
                    onclick={() => { window.location.href = currentError.secondaryAction!.url; }}
                    class={BUTTON_STYLES[currentError.secondaryAction.style || 'secondary']}
                  >
                    {currentError.secondaryAction.label}
                  </button>
                {:else}
                  <a
                    href={currentError.secondaryAction.url}
                    class={BUTTON_STYLES[currentError.secondaryAction.style || 'secondary']}
                  >
                    {currentError.secondaryAction.label}
                  </a>
                {/if}
              {/if}
            </div>
          </div>

          <div
            class="mt-8 flex justify-center text-xs text-slate-400 dark:text-navy-300"
          >
            <a href="https://raw.githubusercontent.com/Alaydriem/bedrock-voice-chat/refs/heads/master/PRIVACY_STATEMENT.md">Privacy Notice</a>
            <div class="mx-3 my-1 w-px bg-slate-200 dark:bg-navy-500"></div>
          </div>
        </div>
      </main>
    </div>