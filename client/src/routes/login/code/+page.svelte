<script lang="ts">
  import "../../../css/app.css";
  import LoginCode from "../../../js/app/loginCode.ts";
  import { onMount } from 'svelte';

  onMount(async () => {
    window.App = new LoginCode();
    window.dispatchEvent(new CustomEvent("app:mounted"));
    await window.App.initialize();
    window.App.preloader();

    document.querySelector("#code-login-form")
      ?.addEventListener("submit", (e) => window.App.submit(e));
  });
</script>

<div
  id="root"
  class="min-h-dvh cloak flex items-center justify-center pb-20 bg-slate-50 dark:bg-navy-900"
>
  <main class="w-full">
    <div class="w-full max-w-[26rem] mx-auto p-4 sm:px-5">
      <div class="text-center">
        <img
          class="mx-auto h-32 w-32"
          src="/images/app-logo-transparent.png"
          alt="Bedrock Voice Chat Logo"
        />
        <div class="mt-4">
          <h2 class="text-2xl font-semibold text-slate-600 dark:text-navy-100">
            Code Login
          </h2>
        </div>
      </div>
      <form id="code-login-form">
        <div class="card mt-5 rounded-lg p-5 lg:p-7">
          <div>
            <label class="block">
              <span class="text-slate-300 dark:text-navy-200">Server</span>
              <input
                id="code-server-display"
                class="form-input mt-1.5 w-full rounded-lg border border-slate-300 bg-transparent px-3 py-2 placeholder:text-slate-400/70 dark:border-navy-450 opacity-60"
                type="text"
                readonly
              />
            </label>
          </div>
          <div class="mt-4">
            <label class="block">
              <span class="text-slate-300 dark:text-navy-200">Game</span>
              <select
                id="code-game-select"
                class="form-select mt-1.5 w-full rounded-lg border border-slate-300 bg-transparent px-3 py-2 dark:border-navy-450 hover:border-slate-400 focus:border-primary dark:hover:border-navy-400 dark:focus:border-accent"
              >
                <option value="minecraft">Minecraft</option>
                <option value="hytale">Hytale</option>
              </select>
            </label>
          </div>
          <div class="mt-4">
            <label class="block">
              <span class="text-slate-300 dark:text-navy-200">Gamertag</span>
              <input
                id="code-gamertag-input"
                class="form-input mt-1.5 w-full rounded-lg border border-slate-300 bg-transparent px-3 py-2 placeholder:text-slate-400/70 hover:border-slate-400 focus:border-primary dark:border-navy-450 dark:hover:border-navy-400 dark:focus:border-accent"
                type="text"
                placeholder="Enter your gamertag"
                autocorrect="off"
                autocapitalize="none"
                spellcheck="false"
              />
            </label>
          </div>
          <div class="mt-4">
            <label class="block">
              <span class="text-slate-300 dark:text-navy-200">Code</span>
              <input
                id="code-input"
                class="form-input mt-1.5 w-full rounded-lg border border-slate-300 bg-transparent px-3 py-2 placeholder:text-slate-400/70 hover:border-slate-400 focus:border-primary dark:border-navy-450 dark:hover:border-navy-400 dark:focus:border-accent font-mono tracking-widest text-center text-lg uppercase"
                type="text"
                placeholder="XXXXXXXX"
                maxlength="8"
                autocorrect="off"
                autocapitalize="characters"
                spellcheck="false"
              />
            </label>
          </div>
          <span
            id="code-error-message"
            class="text-tiny+ text-error invisible mt-2 block"
          ></span>
          <button
            id="code-submit-btn"
            type="submit"
            class="btn mt-5 w-full bg-primary font-medium text-white hover:bg-primary-focus focus:bg-primary-focus active:bg-primary-focus/90 dark:bg-accent dark:hover:bg-accent-focus dark:focus:bg-accent-focus dark:active:bg-accent/90"
          >
            Login
          </button>
          <div class="mt-4 text-center">
            <a
              href="/login"
              class="text-sm text-slate-400 hover:text-slate-500 dark:text-navy-300 dark:hover:text-navy-200 hover:underline"
            >
              Back to login
            </a>
          </div>
        </div>
      </form>
    </div>
  </main>
</div>
