<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from "@tauri-apps/api/core";
  import { error } from '@tauri-apps/plugin-log';
  import ImageCache from "../js/app/components/imageCache";
  import ImageCacheOptions from "../js/app/components/imageCacheOptions";
  import Keyring from "../js/app/keyring.ts";
  import { type LoginResponse } from "../js/bindings/LoginResponse";
  import { type ApiConfig } from "../js/bindings/ApiConfig";

  interface ConfigResponse {
    config: ApiConfig;
    client_version: string;
    compatible: boolean;
    client_too_old: boolean;
  }

  export let id: string;
  export let server: string;

  let buttonDisabled = true;
  let buttonClasses = "bg-primary text-grey";
  let buttonMessage = "Checking Server";
  let showSpinner = true;
  let versionMismatch = false;
  let clientTooOld = false;

  const canvasUrl = `${server}/assets/canvas.png`;
  const avatarUrl = `${server}/assets/avatar.png`;

  const defaultTtl = 60 * 60 * 24 * 7;
  const imageCacher = new ImageCache();
  const canvasImageCacheOptions = new ImageCacheOptions(canvasUrl, defaultTtl);
  const avatarImageCacheOptions = new ImageCacheOptions(avatarUrl, defaultTtl);

  imageCacher.getImage(canvasImageCacheOptions).then((image) => {
    document.getElementById(id)?.querySelector("#canvas-logo")?.setAttribute("src", image);
  });
  imageCacher.getImage(avatarImageCacheOptions).then((image) => {
    document.getElementById(id)?.querySelector("#avatar-logo")?.setAttribute("src", image);
  });

  onMount(async () => {
    await checkServer();
  });

  async function checkServer() {
    try {
      // Get credentials from keyring
      const keyring = await Keyring.new("servers");
      await keyring.setServer(server);

      const credentials = await getCredentials(keyring);

      if (!credentials) {
        showReauthButton();
        return;
      }

      // Initialize client for THIS server
      const cert = typeof credentials.certificate_ca === 'string'
        ? credentials.certificate_ca
        : new TextDecoder().decode(credentials.certificate_ca);

      const certKeyStr = typeof credentials.certificate_key === 'string'
        ? credentials.certificate_key
        : new TextDecoder().decode(credentials.certificate_key);

      const certStr = typeof credentials.certificate === 'string'
        ? credentials.certificate
        : new TextDecoder().decode(credentials.certificate);

      const pem = certStr + certKeyStr;

      await invoke("api_initialize_client", {
        endpoint: server,
        cert: cert,
        pem: pem
      });

      // Get config from THIS specific server and check version compatibility
      const configResponse = await invoke<ConfigResponse>("api_get_config", { server: server });

      if (!configResponse.compatible) {
        versionMismatch = true;
        clientTooOld = configResponse.client_too_old;
        showVersionMismatchButton(configResponse.client_too_old, configResponse.config.protocol_version, configResponse.client_version);
        return;
      }

      showConnectButton();

    } catch (e) {
      error(`Failed to check server ${server}: ${e}`);
      showReauthButton();
    }
  }

  async function getCredentials(keyring: Keyring): Promise<LoginResponse | null> {
    const response: LoginResponse = {} as LoginResponse;
    const keys: (keyof LoginResponse)[] = [
      'gamerpic', 'gamertag', 'keypair', 'signature',
      'certificate', 'certificate_key', 'certificate_ca', 'quic_connect_string'
    ];

    for (const key of keys) {
      const storedValue = await keyring.get(key);
      if (key === "keypair" || key === "signature") {
        let valueStr: string;
        if (typeof storedValue === "string") {
          valueStr = storedValue;
        } else if (storedValue instanceof Uint8Array) {
          valueStr = new TextDecoder().decode(storedValue);
        } else {
          return null;
        }
        (response as any)[key] = JSON.parse(valueStr);
      } else {
        (response as any)[key] = storedValue;
      }
    }
    return response;
  }

  function showConnectButton() {
    buttonDisabled = false;
    showSpinner = false;
    buttonClasses = "bg-success text-white";
    buttonMessage = "Connect!";
  }

  function showReauthButton() {
    buttonDisabled = false;
    showSpinner = false;
    buttonClasses = "bg-error text-white";
    buttonMessage = "Re-authenticate";
  }

  function showVersionMismatchButton(clientTooOld: boolean, serverVersion: string, clientVersion: string) {
    buttonDisabled = true;
    showSpinner = false;
    buttonClasses = "bg-warning text-slate-800";
    if (clientTooOld) {
      buttonMessage = `Update Client (${clientVersion} → ${serverVersion})`;
    } else {
      buttonMessage = `Server Outdated`;
    }
  }

  async function handleClick() {
    if (versionMismatch) {
      // Don't allow click when version mismatch
      return;
    }
    if (buttonMessage === "Connect!") {
      const { Store } = await import('@tauri-apps/plugin-store');
      const store = await Store.load("store.json", {
          autoSave: false,
          defaults: {}
      });
      await store.set("current_server", server);
      await store.save();
      window.location.href = `/dashboard?server=${server}`;
    } else {
      window.location.href = `/login?reauth=true&server=${server}`;
    }
  }
</script>

<div id="{id}" class="card">
  <div class="h-24 rounded-t-lg bg-primary dark:bg-accent">
    <img id="canvas-logo" class="h-full w-full rounded-t-lg object-cover object-center" src="" alt="cover">
  </div>
  <div class="px-4 py-2 sm:px-5">
    <div class="flex justify-between space-x-4">
      <div class="avatar -mt-12 size-20">
        <img id="avatar-logo" class="rounded-full border-2 border-white dark:border-navy-700" src="" alt="avatar">
      </div>
    </div>
    <h3
      id="name"
      class="text-center pb-4 pt-2 text-lg font-medium text-slate-700 dark:text-navy-100 truncate"
      title={server}
    >
      {server}
    </h3>
    <div class="flex justify-center space-x-3 py-3">
      <button
        class="btn h-12 {buttonClasses} text-base font-medium hover:opacity-90"
        disabled={buttonDisabled}
        on:click={handleClick}
      >
        {#if showSpinner}
          <div class="spinner size-7 animate-spin rounded-full border-[3px] border-slate-500 border-r-transparent dark:border-navy-300 dark:border-r-transparent"></div>
        {/if}
        <div>{buttonMessage}</div>
      </button>
    </div>
  </div>
</div>