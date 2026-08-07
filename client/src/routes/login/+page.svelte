<script lang="ts">
  import "../../css/app.css";
  import { onMount, onDestroy } from "svelte";
  import Loader from "$radial/components/Loader.svelte";
  import RadFrame from "../../components/shell/RadFrame.svelte";
  import RadScreen from "../../components/shell/RadScreen.svelte";
  import SignInScreen from "../../components/login/SignInScreen.svelte";
  import ConnectErrorScreen from "../../components/login/ConnectErrorScreen.svelte";
  import CodeScreen from "../../components/login/CodeScreen.svelte";
  import IntroScreen from "../../components/login/IntroScreen.svelte";
  import GateScreen from "../../components/login/GateScreen.svelte";
  import NoServerScreen from "../../components/login/NoServerScreen.svelte";
  import InviteMessage from "../../js/app/login/InviteMessage";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import Login, { CONNECTING_STATUS_PHRASES, type LoginState } from "../../js/app/login.ts";
  import LoginCode, { type CodeLoginInput } from "../../js/app/loginCode.ts";
  import LaunchGate from "../../js/app/login/LaunchGate";
  import AddressResolver, { type ResolveVerdict } from "../../js/app/login/AddressResolver";
  import ScreenFlow from "../../js/app/shell/ScreenFlow";
  import Analytics from "../../js/app/analytics";

  const SCREENS = ["intro", "gate", "login", "code", "notyet"] as const;

  let app: Login | null = null;
  let code: LoginCode | null = null;
  let resolver: AddressResolver | null = null;
  let flow: ScreenFlow | null = null;
  const gate = new LaunchGate();
  const unsubs: Array<() => void> = [];

  let codeError = $state("");
  let codeSubmitting = $state(false);
  let codeServer = $state("");

  // View mirrors of the managers' stores.
  let screen = $state("login");
  let appVersion = $state("");
  let serverInput = $state("");
  let isCodeLogin = $state(false);
  let loginState = $state<LoginState>("idle");
  let attemptServer = $state("");
  let isHandoff = $state(false);
  let isFinishing = $state(false);
  let authUrl = $state("");
  let authUrlCopied = $state(false);
  let verdict = $state<ResolveVerdict>({
    state: "editing",
    ring: "empty",
    line: "",
    caption: "RESOLVING",
  });
  let step = $state(1);

  /**
   * The way off this screen, for someone who arrived from a screen they were already using.
   * Null on a cold launch, where sign-in is the destination and there is nothing behind it.
   *
   * A document navigation rather than a client-side one: the way back leads to the roster,
   * which forwards a single saved server straight to its dashboard, and the boot overlay is
   * what covers that second hop.
   */
  let back = $state<{ href: string; label: string } | null>(null);

  // How long the connecting view has been up, so the recovery affordance can appear
  // late rather than immediately. Fifteen seconds sits under Login's 30s finish
  // watchdog, so the user gets an option before the flow gives up on its own.
  const RECOVERY_AFTER_SECONDS = 15;
  let connectingSeconds = $state(0);

  $effect(() => {
    if (loginState !== "connecting") {
      connectingSeconds = 0;
      return;
    }
    const started = Date.now();
    const id = setInterval(() => {
      connectingSeconds = (Date.now() - started) / 1000;
    }, 500);
    return () => clearInterval(id);
  });


  onMount(() => {
    const instance = new Login();
    app = instance;
    window.App = instance;
    window.dispatchEvent(new CustomEvent("app:mounted"));

    resolver = new AddressResolver();
    unsubs.push(resolver.verdict.subscribe((v) => (verdict = v)));

    code = new LoginCode();
    unsubs.push(code.error.subscribe((v) => (codeError = v)));
    unsubs.push(code.isSubmitting.subscribe((v) => (codeSubmitting = v)));

    unsubs.push(instance.appVersionReadable.subscribe((v) => (appVersion = v)));
    unsubs.push(instance.serverInput.subscribe((v) => (serverInput = v)));
    unsubs.push(instance.isCodeLogin.subscribe((v) => (isCodeLogin = v)));
    unsubs.push(instance.loginState.subscribe((v) => (loginState = v)));
    unsubs.push(instance.attemptServer.subscribe((v) => (attemptServer = v)));
    unsubs.push(instance.isHandoff.subscribe((v) => (isHandoff = v)));
    unsubs.push(instance.isFinishing.subscribe((v) => (isFinishing = v)));
    unsubs.push(instance.authUrl.subscribe((v) => (authUrl = v)));
    unsubs.push(instance.attachVisibilityTracking());

    Analytics.track("LoginShown");

    /*
     * Built before anything awaits, so no store read can leave it null — every screen
     * change on this page goes through it, including "What is this?", which is the only
     * route back into the introduction.
     *
     * It opens on sign-in because that is the answer for anyone who has signed in before.
     * The gate below only has to move someone forward into the introduction.
     */
    flow = new ScreenFlow({
      screens: SCREENS,
      initial: "login",
      steps: 4,
      onScreen: (name) => {
        screen = name;
        if (name === "intro") Analytics.track("OnboardingShown");
      },
    });
    unsubs.push(flow.step.subscribe((v) => (step = v)));

    (async () => {
      const params = new URLSearchParams(window.location.search);
      // An unreadable store is not evidence of a first run. Someone who has been through the
      // introduction should not be handed it again because a read failed, so the benefit of
      // the doubt goes to the sign-in screen.
      const seen = await gate.hasSeenOnboarding().catch(() => true);
      const entry = LaunchGate.resolveEntry(seen, params);
      if (entry !== "login") flow?.go(entry);

      const pageState = await instance.initializePage();
      if (pageState.isAddServer) {
        back = { href: pageState.backHref, label: pageState.backLabel };
      }
      if (pageState.prefilledServer) {
        instance.setServerInput(pageState.prefilledServer);
        resolver?.input(pageState.prefilledServer);
      }

      // A callback that completed before this mount — a cold-started OAuth return —
      // leaves its error in the store; surface it now.
      const pendingError = await instance.consumeAuthError();
      if (pendingError) instance.surfaceAuthError(pendingError);

      // Failures from a callback that resolves while this page stays mounted.
      const unwatch = await instance.watchAuthError((message) => {
        instance.consumeAuthError().catch(() => {});
        instance.surfaceAuthError(message);
      });
      unsubs.push(unwatch);

      if (pageState.autoReauth) await instance.attemptAutoReauth();
    })();
  });

  onDestroy(() => {
    app?.teardownLoginFlow();
    resolver?.destroy();
    for (const unsub of unsubs) unsub();
  });

  const INSTALL_GUIDE_URL = "https://bedrockvoicechat.com/wiki";
  const SITE_URL = "https://bedrockvoicechat.com";

  function handleInput(value: string): void {
    app?.setServerInput(value);
    resolver?.input(value);
  }

  async function copyInvite(): Promise<void> {
    Analytics.track("WikiCtaClicked");
    await navigator.clipboard?.writeText(InviteMessage.TEXT).catch(() => {});
  }

  /**
   * The recovery for a browser that never appeared — a default handler that failed
   * silently, a kiosk, a Linux desktop with no xdg-open. Without this the handoff
   * screen is a dead end: the URL exists, it was just never shown to anyone.
   */
  async function copyAuthUrl(): Promise<void> {
    if (!authUrl) return;
    await navigator.clipboard?.writeText(authUrl).catch(() => {});
    authUrlCopied = true;
  }

  /**
   * Typing `code@<host>` is the unadvertised way in. There is no button for it, and it
   * becomes a screen rather than a navigation.
   *
   * The switch happens on the button, not on the keystroke that completes `code@`. That
   * prefix matches as soon as the `@` is typed, so a reactive switch left the screen
   * mid-address — the host after it was never typed, and whatever partial value had been
   * captured became the server for the rest of the flow.
   */
  let codeTarget = $derived(isCodeLogin && app ? app.codeLoginTarget(serverInput) : null);

  function connect(): void {
    if (isCodeLogin) {
      if (!codeTarget) return;
      codeServer = codeTarget;
      code?.setServer(codeTarget);
      flow?.go("code");
      return;
    }
    void app?.runLogin("ms");
  }

  /**
   * What the line under the field says while a `code@` entry is being typed. The probe
   * has nothing to measure until there is a host, and leaving it on "Resolving" would
   * claim it was working on an address nobody had finished entering.
   */
  const CODE_INCOMPLETE: ResolveVerdict = {
    state: "editing",
    ring: "empty",
    line: "○ Add the server address after code@",
    caption: "AWAITING ADDRESS",
  };

  const CODE_READY: ResolveVerdict = {
    state: "ok",
    ring: "lock",
    line: "✓ Continue to enter your code",
    caption: "CODE SIGN-IN",
  };
</script>

<RadFrame>
  {#if loginState === "connecting"}
    <RadScreen label="Connecting">
      <div class="rad-split">
        <div class="rad-visual-pane">
          <Loader
            loading={true}
            phrases={isHandoff && !isFinishing ? [] : CONNECTING_STATUS_PHRASES}
            slowAfterSeconds={4}
          >
            <!--
              The escape hatch. `?logout=true` is read by Login.initializePage, which
              stops the input device, the output device and the network stream, each
              behind its own timeout — bounded because the subsystem that hung is the
              one being torn down. Do not rename the parameter.
            -->
            {#if connectingSeconds >= RECOVERY_AFTER_SECONDS}
              <a class="rad-btn rad-btn--quiet" href="/login?logout=true">
                Sign out and start over
              </a>
            {/if}
          </Loader>
        </div>
        <div class="rad-content-pane">
          <span class="rad-label rad-rise" style="--d: 50">Connecting</span>
          <h2 class="rad-display rad-rise" style="--d: 120; margin-top: 12px; font-size: 2rem">
            {#if isHandoff && !isFinishing}
              Continue in the window<br /><b>that just opened.</b>
            {:else}
              Reaching<br /><b>{attemptServer}</b>
            {/if}
          </h2>
          <div class="rad-rise" style="--d: 260; margin-top: 22px">
            <button class="rad-btn rad-btn--lg" onclick={() => app?.returnToIdle()}>Cancel</button>
          </div>
          {#if isHandoff && !isFinishing && authUrl}
            <div class="rad-rise" style="--d: 320; margin-top: 18px">
              <button class="rad-btn rad-btn--quiet" onclick={copyAuthUrl}>
                {authUrlCopied ? "Link copied" : "Browser didn't open? Copy this link"}
              </button>
            </div>
          {/if}
        </div>
      </div>
      {#snippet footbar()}
        <span class="rad-label">Signing in with the account you play on</span>
        <span class="rad-label rad-num">v{appVersion}</span>
      {/snippet}
    </RadScreen>
  {:else if screen === "intro"}
    <!--
      Leaving the introduction is what retires it, whether that was the last step or the
      skip. Opening it is not: quitting the app part-way through should bring it back.
    -->
    <IntroScreen
      {step}
      onstep={(n) => flow?.goStep(n)}
      onnext={() => {
        if (!flow?.isLastStep()) {
          flow?.nextStep();
          return;
        }
        void gate.markSeen();
        flow?.go("gate");
      }}
      onback={() => flow?.backStep()}
      onskip={() => {
        void gate.markSeen();
        flow?.go("gate");
      }}
    />
  {:else if screen === "gate"}
    <GateScreen
      onhaveserver={() => flow?.go("login")}
      onnoserver={() => {
        Analytics.track("NoServerClicked");
        flow?.go("notyet");
      }}
    />
  {:else if screen === "notyet"}
    <NoServerScreen
      onguide={() => {
        Analytics.track("WikiCtaClicked");
        void openUrl(INSTALL_GUIDE_URL);
      }}
      oncopyinvite={() => void copyInvite()}
      onwatch={() => {
        Analytics.track("HostingCtaClicked");
        void openUrl(SITE_URL);
      }}
      onwiki={() => {
        Analytics.track("WikiCtaClicked");
        void app?.openWiki();
      }}
      ondiscord={() => void app?.openDiscord()}
      onsignin={() => flow?.go("login")}
    />
  {:else if screen === "code"}
    <CodeScreen
      server={codeServer}
      error={codeError}
      isSubmitting={codeSubmitting}
      {appVersion}
      onsubmit={(input: CodeLoginInput) => void code?.submit(input)}
      onback={() => {
        app?.setServerInput(codeServer);
        flow?.go("login");
        screen = "login";
      }}
    />
  {:else if loginState === "error"}
    <ConnectErrorScreen
      server={attemptServer}
      {appVersion}
      onretry={() => app?.tryAgain()}
      onchangeserver={() => app?.returnToIdle()}
      onwiki={() => app?.openWiki()}
      ondiscord={() => app?.openDiscord()}
    />
  {:else}
    <SignInScreen
      address={serverInput}
      verdict={isCodeLogin ? (codeTarget ? CODE_READY : CODE_INCOMPLETE) : verdict}
      isCode={isCodeLogin}
      {appVersion}
      backLabel={back?.label}
      oninput={handleInput}
      onconnect={connect}
      onprivacy={() => app?.openPrivacyNotice()}
      onrevisit={() => flow?.go("intro")}
      onback={back ? () => (window.location.href = back!.href) : undefined}
    />
  {/if}
</RadFrame>
