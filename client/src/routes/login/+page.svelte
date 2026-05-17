<script lang="ts">
  import "../../css/app.css";
  import Login from "../../js/app/login.ts";
  import { onMount, onDestroy } from 'svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';

  let isMobile = $state(false);
  let appVersion = $state("");
  let formError = $state("");
  let serverInputInvalid = $state(false);
  let isCodeLogin = $state(false);
  let isAddServer = $state(false);
  let backHref = $state("/dashboard");
  let backLabel = $state("Back to Dashboard");
  let serverInputValue = $state("");

  let appInstance: Login | null = $state(null);
  const unsubs: Array<() => void> = [];

  onMount(() => {
    const instance = new Login();
    appInstance = instance;
    window.App = instance;
    window.dispatchEvent(new CustomEvent("app:mounted"));

    unsubs.push(instance.isMobileReadable.subscribe((v) => { isMobile = v; }));
    unsubs.push(instance.appVersionReadable.subscribe((v) => { appVersion = v; }));
    unsubs.push(instance.formError.subscribe((v) => { formError = v; }));
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
        await handleSubmit();
      }
    })();
  });

  onDestroy(() => {
    for (const unsub of unsubs) unsub();
  });

  async function handleSubmit(event?: Event) {
    event?.preventDefault();
    if (!appInstance) return;
    const result = await appInstance.login(serverInputValue);
    if (result.sanitized) {
      serverInputValue = result.sanitized;
    }
  }

  async function handleHytaleLogin(event: Event) {
    event.preventDefault();
    if (!appInstance) return;
    const result = await appInstance.loginWithHytale(serverInputValue);
    if (result.sanitized) {
      serverInputValue = result.sanitized;
    }
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
          {#if isAddServer}
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
          <form onsubmit={handleSubmit}>
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
