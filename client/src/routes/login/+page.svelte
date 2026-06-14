<script lang="ts">
  import "../../css/app.css";
  import Login, { CONNECTING_STATUS_PHRASES } from "../../js/app/login.ts";
  import type { LoginAttemptResult } from "../../js/app/login.ts";
  import { onMount, onDestroy } from 'svelte';
  import { fade } from 'svelte/transition';
  import { openUrl } from '@tauri-apps/plugin-opener';

  // External help links surfaced on the connection-error view.
  const WIKI_URL = "https://github.com/alaydriem/bedrock-voice-chat";
  const DISCORD_URL = "https://discord.gg/WGXy5kBP9E";

  type LoginState = 'idle' | 'connecting' | 'error';
  type LoginKind = 'ms' | 'hytale';

  let isMobile = $state(false);
  let appVersion = $state("");
  let formError = $state("");
  let serverInputInvalid = $state(false);
  let isCodeLogin = $state(false);
  let isAddServer = $state(false);
  let backHref = $state("/dashboard");
  let backLabel = $state("Back to Dashboard");
  let serverInputValue = $state("");

  // Connection flow state machine.
  let loginState = $state<LoginState>('idle');
  let attemptServer = $state("");      // server shown in connecting/error views
  let isHandoff = $state(false);       // success: external auth URL opened
  let lastLoginKind = $state<LoginKind>('ms');
  let fidgetIndex = $state(0);
  // Monotonic token identifying the current attempt. Bumped on each new
  // attempt and on cancel so a late-resolving request can't clobber the UI.
  let attemptId = 0;

  let appInstance: Login | null = $state(null);
  const unsubs: Array<() => void> = [];

  // Cycle the "connecting" status phrases while an attempt is genuinely in
  // flight. Stops once we hand off to the browser or settle into idle/error.
  $effect(() => {
    if (loginState !== 'connecting' || isHandoff) return;
    fidgetIndex = 0;
    const id = setInterval(() => {
      fidgetIndex = (fidgetIndex + 1) % CONNECTING_STATUS_PHRASES.length;
    }, 1600);
    return () => clearInterval(id);
  });

  onMount(() => {
    const instance = new Login();
    appInstance = instance;
    window.App = instance;
    window.dispatchEvent(new CustomEvent("app:mounted"));

    unsubs.push(instance.isMobileReadable.subscribe((v) => { isMobile = v; }));
    unsubs.push(instance.appVersionReadable.subscribe((v) => { appVersion = v; }));
    unsubs.push(instance.formError.subscribe((v) => {
      formError = v;
      // A non-empty error arriving mid-flow (e.g. a Hytale device-flow poll
      // expiring or being denied after we've handed off to the browser) must
      // not be swallowed by the connecting view - drop back to idle so the
      // inline message is actually visible.
      if (v && loginState !== 'idle') {
        isHandoff = false;
        loginState = 'idle';
      }
    }));
    unsubs.push(instance.serverInputInvalid.subscribe((v) => { serverInputInvalid = v; }));

    (async () => {
      const pageState = await instance.initializePage();
      isAddServer = pageState.isAddServer;
      backHref = pageState.backHref;
      backLabel = pageState.backLabel;
      if (pageState.prefilledServer) {
        serverInputValue = pageState.prefilledServer;
        isCodeLogin = Login.isCodeLoginInput(serverInputValue);
      }
      if (pageState.autoReauth) {
        await runLogin('ms');
      }
    })();
  });

  onDestroy(() => {
    for (const unsub of unsubs) unsub();
  });

  function applyResult(result: LoginAttemptResult): void {
    switch (result.status) {
      case 'redirecting':
        // Auth URL opened in the system browser; hold the connecting view.
        isHandoff = true;
        break;
      case 'error':
        attemptServer = result.sanitized;
        loginState = 'error';
        break;
      case 'invalid':
        // Inline validation error is already shown; stay on idle.
        loginState = 'idle';
        break;
      case 'navigating':
        // Same-window navigation in progress; nothing to do.
        break;
    }
  }

  async function runLogin(kind: LoginKind): Promise<void> {
    if (!appInstance || loginState === 'connecting') return;

    lastLoginKind = kind;
    const value = loginState === 'error' ? attemptServer : serverInputValue;

    // Code-login navigates away in the same window - handle it like the
    // dedicated button and never enter the connecting view (otherwise the
    // 'navigating' result would leave us stuck on the spinner).
    if (Login.isCodeLoginInput(value)) {
      window.location.href = appInstance.handleCodeLoginNavigate(value);
      return;
    }

    // Validate before flipping to the connecting view so bad/empty input shows
    // the inline error without a connecting flicker.
    const validation = appInstance.validateServerUrl(value);
    if (!validation.valid) {
      appInstance.reportError(validation.error || "Please enter a valid server URL");
      loginState = 'idle';
      return;
    }

    appInstance.clearError();
    attemptServer = appInstance.sanitizeServerUrl(value);
    serverInputValue = attemptServer;
    isHandoff = false;
    loginState = 'connecting';

    const myAttempt = ++attemptId;
    const result = kind === 'hytale'
      ? await appInstance.loginWithHytale(value)
      : await appInstance.login(value);

    // The user cancelled (or started a fresh attempt) while this one was in
    // flight - don't reapply its result. Undo any polling it may have kicked off.
    if (myAttempt !== attemptId) {
      appInstance.cancelHytalePolling();
      return;
    }

    applyResult(result);
  }

  async function handleSubmit(event?: Event) {
    event?.preventDefault();
    await runLogin('ms');
  }

  async function handleHytaleLogin(event: Event) {
    event.preventDefault();
    await runLogin('hytale');
  }

  async function handleTryAgain() {
    await runLogin(lastLoginKind);
  }

  function handleDifferentServer() {
    returnToIdle();
  }

  // Manual escape from the connecting/handoff view (e.g. the user closed the
  // external sign-in window without finishing). Stops any Hytale polling and
  // clears transient errors before returning to the editable idle form.
  function handleCancelConnecting() {
    returnToIdle();
  }

  function returnToIdle() {
    attemptId++;
    appInstance?.cancelHytalePolling();
    appInstance?.clearError();
    isHandoff = false;
    loginState = 'idle';
    serverInputValue = attemptServer;
  }

  function handleServerInputChange(value: string) {
    serverInputValue = value;
    isCodeLogin = Login.isCodeLoginInput(value);
  }

  function handleServerInputBlur() {
    if (serverInputValue.trim() && appInstance) {
      serverInputValue = appInstance.sanitizeServerUrl(serverInputValue);
    }
  }

  function handleCodeLoginClick() {
    if (!appInstance) return;
    window.location.href = appInstance.handleCodeLoginNavigate(serverInputValue);
  }
</script>

<div
      id="root"
      class="min-h-dvh cloak flex items-center justify-center pb-20 bg-slate-50 dark:bg-navy-900"
    >
      <main class="w-full">
        <div class="w-full max-w-[26rem] mx-auto p-4 sm:px-5">
          {#if isAddServer && loginState === 'idle'}
            <div class="mb-4">
              <a
                href={backHref}
                class="inline-flex items-center space-x-2 text-sm text-slate-500 hover:text-primary dark:text-navy-200 dark:hover:text-accent-light transition-colors"
              >
                <svg xmlns="http://www.w3.org/2000/svg" class="size-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
                </svg>
                <span>{backLabel}</span>
              </a>
            </div>
          {/if}
          <div class="text-center">
            <img
              class="mx-auto h-32 w-32"
              src="/images/app-logo-transparent.png"
              alt="Bedrock Voice Chat Logo"
            />
            <div class="mt-4">
              <h2 class="text-2xl font-semibold text-slate-600 dark:text-navy-100">
                Bedrock Voice Chat
              </h2>
            </div>
          </div>

          {#if loginState === 'idle'}
            <form onsubmit={handleSubmit} in:fade={{ duration: 200 }}>
              <div class="card mt-5 rounded-lg p-5 lg:p-7">
                <div>
                  <p class="text-slate-300 dark:text-navy-200 pb-3">
                    Set your server, then sign in with your account
                  </p>
                  <label class="mt-1.5 flex -space-x-px">
                    <span class="flex items-center justify-center rounded-l-lg border border-slate-300 px-3.5 font-inter dark:border-navy-450">
                      <span class="-mt-1"><i class="fa-solid fa-server" style="color: #ffffff"></i></span>
                    </span>
                    <input
                      class="form-input w-full rounded-r-lg border bg-transparent px-3 py-2 placeholder:text-slate-400/70 hover:z-10 focus:z-10 dark:hover:border-navy-400 dark:focus:border-accent
                        {serverInputInvalid ? 'border-error' : 'border-slate-300 hover:border-slate-400 focus:border-primary dark:border-navy-450'}"
                      placeholder="bvc.alaydriem.com"
                      type="text"
                      autocorrect="off"
                      autocapitalize="none"
                      spellcheck="false"
                      autocomplete="url"
                      value={serverInputValue}
                      oninput={(e) => handleServerInputChange((e.currentTarget as HTMLInputElement).value)}
                      onblur={handleServerInputBlur}
                    />
                  </label>
                  <span class="text-tiny+ text-error mt-2 block {formError ? '' : 'invisible'}">
                    {formError || 'Unable to connect and verify BVC Server. Check the URL?'}
                  </span>
                </div>
                {#if isCodeLogin}
                  <button
                    type="button"
                    class="btn mt-5 w-full bg-primary font-medium text-white hover:bg-primary-focus focus:bg-primary-focus active:bg-primary-focus/90 dark:bg-accent dark:hover:bg-accent-focus dark:focus:bg-accent-focus dark:active:bg-accent/90"
                    onclick={handleCodeLoginClick}
                  >
                    Login with Code
                  </button>
                {:else}
                  <button type="submit" class="btn mt-5 w-full">
                    <img
                      src="/images/ms-symbollockup_signin_dark.svg"
                      alt="Sign in with Microsoft Account"
                      width="215"
                      height="41"
                    />
                  </button>

                  {#if !isMobile}
                    <div class="flex items-center my-4">
                      <hr class="flex-grow border-slate-300 dark:border-navy-450" />
                      <span class="px-3 text-slate-400 dark:text-navy-300 text-sm">or</span>
                      <hr class="flex-grow border-slate-300 dark:border-navy-450" />
                    </div>
                    <button type="button" class="btn w-full" onclick={handleHytaleLogin}>
                      <img
                        src="/images/hytale-login-button.svg"
                        alt="Sign in with Hytale"
                        width="215"
                        height="41"
                      />
                    </button>
                  {/if}
                {/if}
              </div>
            </form>

          {:else if loginState === 'connecting'}
            <div class="card mt-5 rounded-lg p-5 lg:p-7 text-center" in:fade={{ duration: 220 }} role="status" aria-live="polite">
              {#if isHandoff}
                <div class="bvc-ring bvc-ring--success mx-auto"></div>
                <p class="mt-4 text-sm font-medium text-slate-600 dark:text-navy-50">
                  Connected — opening sign-in…
                </p>
                <p class="mt-1 text-tiny+ text-slate-400 dark:text-navy-300 font-inter">
                  Continue in the window that just opened.
                </p>
              {:else}
                <div class="bvc-ring mx-auto"></div>
                <p class="mt-4 text-sm font-semibold text-slate-600 dark:text-navy-50">
                  Connecting to<br />
                  <span class="break-all text-primary dark:text-accent-light">{attemptServer}</span>…
                </p>
                <div class="mt-4 flex items-center justify-center gap-2.5 min-h-[22px] font-inter">
                  <svg class="bvc-sparkle size-[18px] text-primary dark:text-accent-light" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
                    <path d="M12 0l2.2 7.2L21 9.4l-5.6 3.4L17 21l-5-4.3L7 21l1.6-8.2L3 9.4l6.8-2.2z" />
                  </svg>
                  <span class="bvc-shimmer text-xs+">{CONNECTING_STATUS_PHRASES[fidgetIndex]}</span>
                </div>
              {/if}
              <button
                type="button"
                class="mt-5 text-tiny+ text-slate-400 hover:text-slate-500 hover:underline dark:text-navy-300 dark:hover:text-navy-200"
                onclick={handleCancelConnecting}
              >
                Cancel
              </button>
            </div>

          {:else}
            <div class="card mt-5 rounded-lg p-5 lg:p-7 text-center" in:fade={{ duration: 220 }}>
              <div class="mx-auto flex size-14 items-center justify-center rounded-full bg-error/10 text-error">
                <i class="fa-solid fa-triangle-exclamation text-2xl"></i>
              </div>
              <h3 class="mt-4 text-base font-semibold text-slate-600 dark:text-navy-50">
                We couldn’t connect
              </h3>
              <p class="mt-2.5 text-sm leading-relaxed text-slate-500 dark:text-navy-200 font-inter">
                Attempting to connect to your Bedrock Voice Chat Server at
                <span class="font-semibold break-all text-primary dark:text-accent-light">{attemptServer}</span>
                wasn’t successful. Make sure your BVC server is up and running, and that you have permissions to the server.
              </p>

              <button
                type="button"
                class="btn mt-5 w-full bg-primary font-medium text-white hover:bg-primary-focus focus:bg-primary-focus active:bg-primary-focus/90 dark:bg-accent dark:hover:bg-accent-focus dark:focus:bg-accent-focus dark:active:bg-accent/90"
                onclick={handleTryAgain}
              >
                Try again
              </button>
              <button
                type="button"
                class="btn mt-3 w-full border border-slate-300 font-medium text-slate-800 hover:bg-slate-150 focus:bg-slate-150 active:bg-slate-150/80 dark:border-navy-450 dark:text-navy-50 dark:hover:bg-navy-500 dark:focus:bg-navy-500 dark:active:bg-navy-500/90"
                onclick={handleDifferentServer}
              >
                Connect to a different server
              </button>

              <div class="mt-5 border-t border-slate-200 pt-4 dark:border-navy-500">
                <p class="text-tiny+ text-slate-400 dark:text-navy-300 font-inter">Still stuck?</p>
                <div class="mt-2 flex items-center justify-center gap-5 font-inter">
                  <button
                    type="button"
                    class="inline-flex items-center gap-1.5 text-xs text-primary hover:underline dark:text-accent-light"
                    onclick={() => openUrl(WIKI_URL)}
                  >
                    <i class="fa-solid fa-book"></i> Read the wiki
                  </button>
                  <button
                    type="button"
                    class="inline-flex items-center gap-1.5 text-xs text-primary hover:underline dark:text-accent-light"
                    onclick={() => openUrl(DISCORD_URL)}
                  >
                    <i class="fa-brands fa-discord"></i> Ask on Discord
                  </button>
                </div>
              </div>
            </div>
          {/if}

          <div class="mt-8 flex flex-col items-center text-xs text-slate-400 dark:text-navy-300">
            <button
              type="button"
              class="hover:text-slate-500 dark:hover:text-navy-200 hover:underline cursor-pointer"
              onclick={() => openUrl("https://raw.githubusercontent.com/Alaydriem/bedrock-voice-chat/refs/heads/master/PRIVACY_STATEMENT.md")}
            >Privacy Notice</button>
            {#if appVersion}
              <span class="mt-2">v{appVersion}</span>
            {/if}
          </div>
        </div>
      </main>
    </div>

<style>
  .bvc-ring {
    width: 46px;
    height: 46px;
    border-radius: 9999px;
    border: 3px solid rgb(148 163 184 / 0.35);
    border-top-color: #5f5af6;
    animation: bvc-spin 0.9s linear infinite;
  }
  .bvc-ring--success {
    border-top-color: #10b981;
    animation: none;
  }
  @keyframes bvc-spin {
    to { transform: rotate(360deg); }
  }

  .bvc-sparkle {
    animation: bvc-sparkle 3.2s ease-in-out infinite;
  }
  @keyframes bvc-sparkle {
    0% { transform: rotate(0) scale(0.85); opacity: 0.7; }
    50% { transform: rotate(180deg) scale(1.1); opacity: 1; }
    100% { transform: rotate(360deg) scale(0.85); opacity: 0.7; }
  }

  .bvc-shimmer {
    background: linear-gradient(
      90deg,
      currentColor 0%,
      currentColor 35%,
      #ffffff 50%,
      currentColor 65%,
      currentColor 100%
    );
    color: rgb(100 116 139);
    background-size: 200% 100%;
    -webkit-background-clip: text;
    background-clip: text;
    -webkit-text-fill-color: transparent;
    animation: bvc-shimmer 2.2s linear infinite;
  }
  :global(.dark) .bvc-shimmer {
    color: #697a9b;
  }
  @keyframes bvc-shimmer {
    0% { background-position: 120% 0; }
    100% { background-position: -120% 0; }
  }

  @media (prefers-reduced-motion: reduce) {
    .bvc-ring { animation-duration: 1.6s; }
    .bvc-sparkle,
    .bvc-shimmer { animation: none; }
    .bvc-shimmer { -webkit-text-fill-color: currentColor; }
  }
</style>
