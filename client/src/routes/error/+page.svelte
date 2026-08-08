<script lang="ts">
  import "../../css/app.css";
  import BootOverlay from "../../js/app/shell/BootOverlay";
  import { I18n } from "$lib/i18n";
  import RadFrame from "../../components/shell/RadFrame.svelte";
  import FaultScreen from "../../components/error/FaultScreen.svelte";
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { getVersion } from "@tauri-apps/api/app";
  import { stopForegroundService, isServiceRunning } from "tauri-plugin-audio-permissions";
  import { Store } from "@tauri-apps/plugin-store";
  import FaultCatalog from "../../js/app/error/FaultCatalog";
  import HelpLinks from "../../js/app/HelpLinks";
  import PlatformDetector from "../../js/app/utils/PlatformDetector";
  import { AppStore } from "../../js/app/services/AppStore";

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

  /**
   * The inputs the screen is decided by, rather than the decision itself.
   *
   * Holding the resolved definition would freeze its copy at the moment `onMount` ran,
   * because the strings are read out of the catalog on access. Deriving it instead means a
   * language change re-reads them, and the screen follows.
   */
  let errorCode = $state<string | null>(null);
  let updateVersion = $state<string | null>(null);
  let singleServer = $state(false);

  const currentError = $derived(
    FaultCatalog.forScreen(errorCode, updateVersion, singleServer),
  );

  let appVersion = $state("");

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
    const isUpdate = FaultCatalog.isUpdate(currentError.code);
    const list = [
      {
        label: isUpdate && isUpdating ? I18n.t("Updating…") : currentError.primaryAction.label,
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
    window.dispatchEvent(new CustomEvent("app:mounted"));
    BootOverlay.dismiss();

    try { appVersion = await getVersion(); } catch (_) {}

    const urlParams = new URLSearchParams(window.location.search);
    errorCode = urlParams.get("code");
    updateVersion = urlParams.get("version");
    const isUpdate = FaultCatalog.isUpdate(errorCode);

    // Nothing is streaming when the splash routes here, and the update is the one screen
    // reached before a session exists.
    if (!isUpdate) {
      await teardown();
    }

    // "Stay on This Version" is not a server switch, so the update keeps both of its
    // actions however many servers are configured.
    if (!isUpdate) {
      const store = await AppStore.load();
      const serverList = await store.get("server_list") as Array<{ server: string; player: string }> | null;

      singleServer = serverList == null || serverList.length <= 1;
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
    workingPhrases={FaultCatalog.UPDATE_PHRASES}
  >
    {#snippet footnote()}
      {#if updateError}
        <div class="rad-callout rad-callout--bad rad-rise" style="--d: 340; margin-top: 18px">
          <span>{I18n.t("The update could not be installed. Try again, or download the new version by hand.")}</span>
        </div>
      {:else if currentError.severity !== 'ok'}
        <span class="rad-label rad-rise" style="--d: 360; display: block; margin-top: 26px">
          {I18n.t("Still stuck?")}
        </span>
        <div class="rad-swatchrow rad-rise" style="--d: 390">
          <button class="rad-pill-link" onclick={() => void HelpLinks.openWiki()}>
            {I18n.t("Wiki")} <span class="rad-pill-link__ext">&#8599;</span>
          </button>
          <button class="rad-pill-link" onclick={() => void HelpLinks.openDiscord()}>
            Discord <span class="rad-pill-link__ext">&#8599;</span>
          </button>
        </div>
      {/if}
    {/snippet}
  </FaultScreen>
</RadFrame>
