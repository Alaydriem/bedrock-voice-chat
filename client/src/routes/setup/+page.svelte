<script lang="ts">
  import "../../css/app.css";
  import { onMount, onDestroy } from "svelte";
  import { PermissionType } from "tauri-plugin-audio-permissions";
  import RadFrame from "../../components/shell/RadFrame.svelte";
  import MicrophoneScreen from "../../components/setup/MicrophoneScreen.svelte";
  import NotificationsScreen from "../../components/setup/NotificationsScreen.svelte";
  import DevicesScreen from "../../components/setup/DevicesScreen.svelte";
  import SetupFlow from "../../js/app/setup/SetupFlow";
  import ScreenFlow from "../../js/app/shell/ScreenFlow";
  import PermissionRequestManager, {
    type PermissionFlowState,
  } from "../../js/app/PermissionRequestManager";
  import PlatformDetector from "../../js/app/utils/PlatformDetector";
  import InputMeter from "../../js/app/setup/InputMeter";
  import SpeakerTest from "../../js/app/setup/SpeakerTest";
  import BootOverlay from "../../js/app/shell/BootOverlay";
  import { error } from "@charlesportwoodii/tauri-plugin-curia";

  const SCREENS = ["microphone", "notifications", "devices"] as const;
  const TOTAL = SCREENS.length;

  let setup: SetupFlow | null = null;
  let flow: ScreenFlow | null = null;
  let permissions: PermissionRequestManager | null = null;
  const unsubs: Array<() => void> = [];

  let screen = $state("microphone");
  let permissionState = $state<PermissionFlowState>("idle");
  let ready = $state(false);
  let inputLevel = $state(0);
  let gateOpen = $state(false);
  let meterAvailable = $state(true);
  const speaker = new SpeakerTest();

  let step = $derived(SCREENS.indexOf(screen as (typeof SCREENS)[number]) + 1);


  onMount(() => {
    (async () => {
      setup = new SetupFlow();
      await setup.initialize();

      // Mobile never reaches the device picker: the platform owns routing, so
      // clearing the step here is what stops it stalling on a screen it cannot use.
      if (await new PlatformDetector().checkMobile()) {
        if (!setup.currentState().devices) await setup.completeStep("devices");
      }

      const next = setup.nextScreen();
      if (next === null) {
        // Straight through to the dashboard, which takes the overlay down itself. Doing
        // it here too would fade it out over a page that is about to be replaced.
        setup.reportCompletion();
        // Replaced, not pushed: setup is finished, and back into it re-runs a flow that
        // has nothing left to ask. It also puts the dashboard at the bottom of the stack,
        // which is what lets back there hand the press to the platform.
        window.location.replace("/dashboard");
        return;
      }

      flow = new ScreenFlow({
        screens: SCREENS,
        initial: next,
        onScreen: (name) => (screen = name),
      });
      screen = next;
      ready = true;
      armPermissions(next);

      // Only now, with a screen on. This route has no window.App to call `preloader()`
      // on — SetupFlow is a flow, not an app — so the overlay is taken down here or not
      // at all, and not at all looks exactly like the app hanging on launch.
      BootOverlay.dismiss();
    })().catch((e) => {
      // An unhandled rejection here leaves `ready` false, so nothing renders and the
      // boot overlay stays up over an empty page. Logging and leaving the overlay in
      // place is deliberate: its escape hatch is the only way out of this state, and
      // dismissing it would replace a recoverable stall with a blank screen.
      void error(`Setup failed to initialize: ${e}`);
    });
  });

  // The screen holds no listener of its own: the level arrives here and goes down as
  // a plain number, which is what keeps the screen testable without Tauri. The meter
  // lives only as long as the device screen is on, so no capture stream outlives the
  // step that needs it.
  $effect(() => {
    if (screen !== "devices") return;
    const meter = new InputMeter((level) => {
      inputLevel = level.rms;
      gateOpen = level.gate_open;
    });
    void meter.start().then((started) => (meterAvailable = started));
    return () => {
      inputLevel = 0;
      gateOpen = false;
      meterAvailable = true;
      void meter.stop();
    };
  });

  onDestroy(() => {
    permissions?.destroy();
    for (const unsub of unsubs) unsub();
  });

  /**
   * A permission screen re-checks the OS on arrival, so a step already granted
   * resolves and advances without asking again. That is what makes re-running setup
   * on every launch cheap.
   */
  function armPermissions(name: string): void {
    permissions?.destroy();
    permissions = null;
    permissionState = "idle";

    const type =
      name === "microphone"
        ? PermissionType.Audio
        : name === "notifications"
          ? PermissionType.Notification
          : null;
    if (type === null) return;

    const manager = new PermissionRequestManager(type);
    permissions = manager;
    unsubs.push(
      manager.state.subscribe((s) => {
        permissionState = s;
        if (s === "granted") void complete(name);
      }),
    );
    void manager.start();
  }

  async function complete(name: string): Promise<void> {
    if (!setup || !flow) return;
    await setup.completeStep(name as "microphone" | "notifications" | "devices");

    const next = setup.nextScreen();
    if (next === null) {
      setup.reportCompletion();
      window.location.replace("/dashboard");
      return;
    }
    flow.go(next);
    armPermissions(next);
  }
</script>

{#if ready}
  <RadFrame>
    {#if screen === "microphone"}
      <MicrophoneScreen
        state={permissionState}
        {step}
        total={TOTAL}
        onrequest={() => void permissions?.requestPermission()}
      />
    {:else if screen === "notifications"}
      <NotificationsScreen
        state={permissionState}
        {step}
        total={TOTAL}
        onrequest={() => void permissions?.requestPermission()}
      />
    {:else}
      <DevicesScreen
        {step}
        total={TOTAL}
        {inputLevel}
        {gateOpen}
        available={meterAvailable}
        ontestspeaker={() => speaker.play()}
        oncontinue={() => void complete("devices")}
      />
    {/if}
  </RadFrame>
{/if}
