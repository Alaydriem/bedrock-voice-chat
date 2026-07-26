<script lang="ts">
  import "../../css/app.css";
  import Login, { CONNECTING_STATUS_PHRASES, BRAILLE_FRAMES, type LoginState } from "../../js/app/login.ts";
  import { onMount, onDestroy } from 'svelte';
  import { fade } from 'svelte/transition';

  let appInstance: Login | null = $state(null);
  const unsubs: Array<() => void> = [];

  // View mirrors of the manager's stores.
  let isMobile = $state(false);
  let appVersion = $state("");
  let formError = $state("");
  let serverInputInvalid = $state(false);
  let serverInputValue = $state("");
  let isCodeLogin = $state(false);
  let loginState = $state<LoginState>('idle');
  let attemptServer = $state("");
  let isHandoff = $state(false);
  let isFinishing = $state(false);

  // Page chrome resolved once on mount.
  let isAddServer = $state(false);
  let backHref = $state("/dashboard");
  let backLabel = $state("Back to Dashboard");

  // Presentation-only animation state for the connecting view.
  let fidgetIndex = $state(0);
  let spinnerFrame = $state(0);
  let ringAngle = $state(0);

  // Drive the spinner from JS so motion survives Android WebView settings that
  // suppress CSS animations (reduced-motion / animator duration scale = 0). Runs
  // while genuinely connecting and while finalizing after the browser handoff.
  $effect(() => {
    const moving = loginState === 'connecting' && (!isHandoff || isFinishing);
    if (!moving) return;

    let tick = 0;
    const motionId = setInterval(() => {
      ringAngle = (ringAngle + 14) % 360;
      tick = (tick + 1) % 3;
      if (tick === 0) spinnerFrame = (spinnerFrame + 1) % BRAILLE_FRAMES.length;
    }, 40);

    // The cycling status copy only makes sense before the handoff.
    let phraseId: ReturnType<typeof setInterval> | null = null;
    if (!isFinishing) {
      fidgetIndex = 0;
      phraseId = setInterval(() => {
        fidgetIndex = (fidgetIndex + 1) % CONNECTING_STATUS_PHRASES.length;
      }, 1600);
    }

    return () => {
      clearInterval(motionId);
      if (phraseId) clearInterval(phraseId);
    };
  });

  onMount(() => {
    const instance = new Login();
    appInstance = instance;
    window.App = instance;
    window.dispatchEvent(new CustomEvent("app:mounted"));

    unsubs.push(instance.isMobileReadable.subscribe((v) => { isMobile = v; }));
    unsubs.push(instance.appVersionReadable.subscribe((v) => { appVersion = v; }));
    unsubs.push(instance.formError.subscribe((v) => { formError = v; }));
    unsubs.push(instance.serverInputInvalid.subscribe((v) => { serverInputInvalid = v; }));
    unsubs.push(instance.serverInput.subscribe((v) => { serverInputValue = v; }));
    unsubs.push(instance.isCodeLogin.subscribe((v) => { isCodeLogin = v; }));
    unsubs.push(instance.loginState.subscribe((v) => { loginState = v; }));
    unsubs.push(instance.attemptServer.subscribe((v) => { attemptServer = v; }));
    unsubs.push(instance.isHandoff.subscribe((v) => { isHandoff = v; }));
    unsubs.push(instance.isFinishing.subscribe((v) => { isFinishing = v; }));

    unsubs.push(instance.attachVisibilityTracking());

    (async () => {
      const pageState = await instance.initializePage();
      isAddServer = pageState.isAddServer;
      backHref = pageState.backHref;
      backLabel = pageState.backLabel;
      if (pageState.prefilledServer) {
        instance.setServerInput(pageState.prefilledServer);
      }

      // A callback that completed before this mount (e.g. a cold-started OAuth
      // return) leaves its error in the store; surface it now.
      const pendingError = await instance.consumeAuthError();
      if (pendingError) {
        instance.surfaceAuthError(pendingError);
      }

      // Observe failures from a callback that resolves while this page stays
      // mounted (the warm OAuth return path).
      const unwatch = await instance.watchAuthError((message) => {
        instance.consumeAuthError().catch(() => {});
        instance.surfaceAuthError(message);
      });
      unsubs.push(unwatch);

      if (pageState.autoReauth) {
        await instance.attemptAutoReauth();
      }
    })();
  });

  onDestroy(() => {
    appInstance?.teardownLoginFlow();
    for (const unsub of unsubs) unsub();
  });

  function handleSubmit(event?: Event) {
    event?.preventDefault();
    appInstance?.runLogin('ms');
  }
</script>

<div
      id="root"
      class="h-full overflow-y-auto cloak flex bg-slate-50 dark:bg-navy-900"
    >
      <main class="w-full m-auto py-8">
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
                      oninput={(e) => appInstance?.setServerInput((e.currentTarget as HTMLInputElement).value)}
                      onblur={() => appInstance?.blurServerInput()}
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
                    onclick={() => appInstance?.navigateCodeLogin()}
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
                {/if}
              </div>
            </form>

          {:else if loginState === 'connecting'}
            <div class="card mt-5 rounded-lg p-5 lg:p-7 text-center" in:fade={{ duration: 220 }} role="status" aria-live="polite">
              {#if isHandoff && !isFinishing}
                <div class="bvc-badge bvc-badge--success mx-auto"><i class="fa-solid fa-check"></i></div>
                <p class="mt-4 text-sm font-medium text-slate-600 dark:text-navy-50">
                  Connected — opening sign-in…
                </p>
                <p class="mt-1 text-tiny+ text-slate-400 dark:text-navy-300 font-inter">
                  Continue in the window that just opened.
                </p>
              {:else if isFinishing}
                <div class="bvc-ring mx-auto" style="transform: rotate({ringAngle}deg)"></div>
                <p class="mt-4 text-sm font-semibold text-slate-600 dark:text-navy-50">
                  Finishing sign-in…
                </p>
                <div class="mt-4 flex items-center justify-center gap-2.5 min-h-[22px] font-inter">
                  <span class="bvc-spinner text-primary dark:text-accent-light" aria-hidden="true">{BRAILLE_FRAMES[spinnerFrame]}</span>
                  <span class="text-xs+ text-slate-400 dark:text-navy-300">Completing authentication…</span>
                </div>
              {:else}
                <div class="bvc-ring mx-auto" style="transform: rotate({ringAngle}deg)"></div>
                <p class="mt-4 text-sm font-semibold text-slate-600 dark:text-navy-50">
                  Connecting to<br />
                  <span class="break-all text-primary dark:text-accent-light">{attemptServer}</span>…
                </p>
                <div class="mt-4 flex items-center justify-center gap-2.5 min-h-[22px] font-inter">
                  <span class="bvc-spinner text-primary dark:text-accent-light" aria-hidden="true">{BRAILLE_FRAMES[spinnerFrame]}</span>
                  <span class="bvc-shimmer text-xs+">{CONNECTING_STATUS_PHRASES[fidgetIndex]}</span>
                </div>
              {/if}
              <button
                type="button"
                class="mt-5 text-tiny+ text-slate-400 hover:text-slate-500 hover:underline dark:text-navy-300 dark:hover:text-navy-200"
                onclick={() => appInstance?.returnToIdle()}
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
                onclick={() => appInstance?.tryAgain()}
              >
                Try again
              </button>
              <button
                type="button"
                class="btn mt-3 w-full border border-slate-300 font-medium text-slate-800 hover:bg-slate-150 focus:bg-slate-150 active:bg-slate-150/80 dark:border-navy-450 dark:text-navy-50 dark:hover:bg-navy-500 dark:focus:bg-navy-500 dark:active:bg-navy-500/90"
                onclick={() => appInstance?.returnToIdle()}
              >
                Connect to a different server
              </button>

              <div class="mt-5 border-t border-slate-200 pt-4 dark:border-navy-500">
                <p class="text-tiny+ text-slate-400 dark:text-navy-300 font-inter">Still stuck?</p>
                <div class="mt-2 flex items-center justify-center gap-5 font-inter">
                  <button
                    type="button"
                    class="inline-flex items-center gap-1.5 text-xs text-primary hover:underline dark:text-accent-light"
                    onclick={() => appInstance?.openWiki()}
                  >
                    <i class="fa-solid fa-book"></i> Read the wiki
                  </button>
                  <button
                    type="button"
                    class="inline-flex items-center gap-1.5 text-xs text-primary hover:underline dark:text-accent-light"
                    onclick={() => appInstance?.openDiscord()}
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
              onclick={() => appInstance?.openPrivacyNotice()}
            >Privacy Notice</button>
            {#if appVersion}
              <span class="mt-2">v{appVersion}</span>
            {/if}
          </div>
        </div>
      </main>
    </div>

<style>
  /* Deliberately overrides the global .bvc-ring: no CSS animation, because
     this page rotates the ring from JS so motion survives Android WebView
     settings that suppress CSS animations. */
  .bvc-ring {
    width: 46px;
    height: 46px;
    border-radius: 9999px;
    border: 3px solid rgb(148 163 184 / 0.35);
    border-top-color: #5f5af6;
    animation: none;
  }
</style>
