<script lang="ts">
  import "../../css/app.css";
  import App from "../../js/app/app.ts";
  import { onMount } from 'svelte';

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

  let currentError = ERROR_DEFINITIONS['DEFAULT'];

  // Button style configurations
  const BUTTON_STYLES = {
    primary: 'btn mt-5 w-full bg-primary font-medium text-white hover:bg-primary-focus focus:bg-primary-focus active:bg-primary-focus/90 dark:bg-accent dark:hover:bg-accent-focus dark:focus:bg-accent-focus dark:active:bg-accent/90',
    secondary: 'btn mt-3 w-full border border-slate-300 font-medium text-slate-800 hover:bg-slate-150 focus:bg-slate-150 active:bg-slate-150/80 dark:border-navy-450 dark:text-navy-50 dark:hover:bg-navy-500 dark:focus:bg-navy-500 dark:active:bg-navy-500/90',
    danger: 'btn mt-3 w-full bg-error font-medium text-white hover:bg-error-focus focus:bg-error-focus active:bg-error-focus/90'
  };

  onMount(() => {
    window.App = new App();
    window.dispatchEvent(new CustomEvent("app:mounted"));
    window.App.preloader();
    // Read error code from query parameter
    const urlParams = new URLSearchParams(window.location.search);
    const errorCode = urlParams.get('code');

    if (errorCode && ERROR_DEFINITIONS[errorCode]) {
      currentError = ERROR_DEFINITIONS[errorCode];
    } else if (errorCode) {
      // If code is provided but not found, use default with the provided code
      currentError = {
        ...ERROR_DEFINITIONS['DEFAULT'],
        code: errorCode
      };
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
                  <i class="{currentError.icon} text-5xl text-error"></i>
                </div>
              {/if}

              <!-- Error Code -->
              <p class="text-5xl font-bold text-primary dark:text-accent">
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

              <!-- Primary Action Button -->
              <a
                href={currentError.primaryAction.url}
                class={BUTTON_STYLES[currentError.primaryAction.style || 'primary']}
              >
                {currentError.primaryAction.label}
              </a>

              <!-- Secondary Action Button (Optional) -->
              {#if currentError.secondaryAction}
                <a
                  href={currentError.secondaryAction.url}
                  class={BUTTON_STYLES[currentError.secondaryAction.style || 'secondary']}
                >
                  {currentError.secondaryAction.label}
                </a>
              {/if}
            </div>
          </div>

          <div
            class="mt-8 flex justify-center text-xs text-slate-400 dark:text-navy-300"
          >
            <a href="#">Privacy Notice</a>
            <div class="mx-3 my-1 w-px bg-slate-200 dark:bg-navy-500"></div>
            <a href="#">Term of service</a>
          </div>
        </div>
      </main>
    </div>