<script lang="ts">
  import "../../../css/app.css";
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { Store } from '@tauri-apps/plugin-store';
  import { info, error as logError } from '@tauri-apps/plugin-log';
  import Keyring from '../../../js/app/keyring.ts';
  import type { LoginResponse } from '../../../js/bindings/LoginResponse';

  let serverUrl = $state("");
  let gamertag = $state("");
  let code = $state("");
  let errorMessage = $state("");
  let isLoading = $state(false);

  onMount(() => {
    const params = new URLSearchParams(window.location.search);
    serverUrl = params.get("server") || "";
  });

  async function handleSubmit(event: Event) {
    event.preventDefault();
    errorMessage = "";
    isLoading = true;

    if (!gamertag.trim() || !code.trim()) {
      errorMessage = "Please enter both your gamertag and code.";
      isLoading = false;
      return;
    }

    try {
      info(`Attempting code login to ${serverUrl}`);

      const response = await invoke<LoginResponse>("code_login", {
        server: serverUrl,
        gamertag: gamertag.trim(),
        code: code.trim().toUpperCase(),
      });

      const store = await Store.load("store.json", { autoSave: false, defaults: {} });
      const keyring = await Keyring.new("servers");
      await keyring.setServer(serverUrl);

      for (const key of Object.keys(response)) {
        const value = response[key as keyof LoginResponse];
        if (value === null || value === undefined) continue;
        if (typeof value === "string" || value instanceof Uint8Array) {
          await keyring.insert(key, value);
        } else {
          await keyring.insert(key, JSON.stringify(value));
        }
      }

      await store.set("current_server", serverUrl);
      await store.set("current_player", response.gamertag);
      await store.set("active_game", "minecraft");

      const serverList = (await store.get("server_list") as Array<{ server: string, player: string, game?: string }>) || [];
      const existing = serverList.find((s) => s.server === serverUrl);
      if (existing) {
        existing.game = "minecraft";
      } else {
        serverList.push({ server: serverUrl, player: response.gamertag, game: "minecraft" });
      }
      await store.set("server_list", serverList);
      await store.save();

      info("Code login successful, navigating to onboarding");
      window.location.href = "/onboarding/welcome";
    } catch (e) {
      logError(`Code login failed: ${String(e)}`);
      errorMessage = "Login failed. Check your code and gamertag.";
    } finally {
      isLoading = false;
    }
  }
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
      <form onsubmit={handleSubmit}>
        <div class="card mt-5 rounded-lg p-5 lg:p-7">
          <div>
            <label class="block">
              <span class="text-slate-300 dark:text-navy-200">Server</span>
              <input
                class="form-input mt-1.5 w-full rounded-lg border border-slate-300 bg-transparent px-3 py-2 placeholder:text-slate-400/70 dark:border-navy-450 opacity-60"
                type="text"
                value={serverUrl}
                readonly
              />
            </label>
          </div>
          <div class="mt-4">
            <label class="block">
              <span class="text-slate-300 dark:text-navy-200">Gamertag</span>
              <input
                class="form-input mt-1.5 w-full rounded-lg border border-slate-300 bg-transparent px-3 py-2 placeholder:text-slate-400/70 hover:border-slate-400 focus:border-primary dark:border-navy-450 dark:hover:border-navy-400 dark:focus:border-accent"
                type="text"
                placeholder="Enter your gamertag"
                bind:value={gamertag}
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
                class="form-input mt-1.5 w-full rounded-lg border border-slate-300 bg-transparent px-3 py-2 placeholder:text-slate-400/70 hover:border-slate-400 focus:border-primary dark:border-navy-450 dark:hover:border-navy-400 dark:focus:border-accent font-mono tracking-widest text-center text-lg uppercase"
                type="text"
                placeholder="XXXXXXXX"
                bind:value={code}
                maxlength="8"
                autocorrect="off"
                autocapitalize="characters"
                spellcheck="false"
              />
            </label>
          </div>
          {#if errorMessage}
            <span class="text-tiny+ text-error mt-2 block">{errorMessage}</span>
          {/if}
          <button
            type="submit"
            class="btn mt-5 w-full bg-primary font-medium text-white hover:bg-primary-focus focus:bg-primary-focus active:bg-primary-focus/90 dark:bg-accent dark:hover:bg-accent-focus dark:focus:bg-accent-focus dark:active:bg-accent/90"
            disabled={isLoading}
          >
            {isLoading ? "Logging in..." : "Login"}
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
