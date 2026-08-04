<script lang="ts">
    import "../../css/app.css";
    import Loader from "$radial/components/Loader.svelte";
    import RadScreen from "../../components/shell/RadScreen.svelte";
    import IntroScreen from "../../components/login/IntroScreen.svelte";
    import GateScreen from "../../components/login/GateScreen.svelte";
    import SignInScreen from "../../components/login/SignInScreen.svelte";
    import CodeScreen from "../../components/login/CodeScreen.svelte";
    import NoServerScreen from "../../components/login/NoServerScreen.svelte";
    import ConnectErrorScreen from "../../components/login/ConnectErrorScreen.svelte";
    import MicrophoneScreen from "../../components/setup/MicrophoneScreen.svelte";
    import NotificationsScreen from "../../components/setup/NotificationsScreen.svelte";
    import DevicesScreen from "../../components/setup/DevicesScreen.svelte";
    import FaultScreen from "../../components/error/FaultScreen.svelte";
    import ServerListScreen from "../../components/server/ServerListScreen.svelte";
    import FaultCatalog from "../../js/app/error/FaultCatalog";
    import type { ResolveVerdict } from "../../js/app/login/AddressResolver";
    import type { RosterStatus } from "../../js/app/server/RosterStatus";
    import type { ServerRosterEntry } from "../../js/app/server/ServerRosterEntry";

    /**
     * Every login and setup screen, at both reference frame sizes, on one page.
     *
     * The kit's responsive rules are container queries against `.rad-frame`, so a
     * 412px frame inside a desktop window lays out exactly as a phone does — which is
     * the whole reason this page can exist. A breakpoint that only reproduced by
     * resizing the window could not be reviewed side by side, and a screen whose
     * footbar falls off the bottom is invisible to any test that has no layout.
     *
     * Dev only, and temporary: it exists to settle a review and then goes.
     */
    const DEV = import.meta.env.DEV;

    const noop = () => {};

    const VERDICTS: Record<string, ResolveVerdict> = {
        ok: { state: "ok", ring: "live", line: "✓ Resolved · 41ms", caption: "REACHABLE" },
        bad: { state: "bad", ring: "empty", line: "✕ Nothing at that address", caption: "NO RESPONSE" },
        editing: { state: "editing", ring: "empty", line: "○ Resolving", caption: "RESOLVING" },
    };

    let introStep = $state(1);

    /**
     * `/error?code=X` reaches any one of these in the real app and is the better way to
     * check a single screen. Two things it cannot reach: the update mid-install, which
     * needs an update to exist and then commits to installing it, and any two states of the
     * same screen at once.
     */
    /**
     * A server list with one row in every state at once, which no real device can produce:
     * it needs a live server, a lapsed sign-in, a dead host and two mismatched protocols
     * saved side by side. Mocking store.json reaches the layout but only ever the failing
     * states, because a made-up host cannot pass a health check.
     */
    function row(host: string, status: RosterStatus, extra: Partial<ServerRosterEntry> = {}) {
        return {
            server: `https://${host}`,
            host,
            player: "Alaydriem",
            game: "minecraft",
            status,
            serverVersion: "",
            clientVersion: "",
            clientTooOld: false,
            isCurrent: false,
            ...extra,
        } as ServerRosterEntry;
    }

    const ROSTER: ServerRosterEntry[] = [
        row("s4.bedrock-legends.bedrockvc.stream", "connect", { isCurrent: true }),
        row("voice.hearthhold.net", "reauth"),
        row("bvc.tinyaxolotl.gg", "unreachable"),
        row("old.example.com", "version_mismatch", {
            serverVersion: "2.0.0",
            clientVersion: "2.1.0",
        }),
        row("ahead.example.com", "version_mismatch", {
            clientTooOld: true,
            serverVersion: "2.2.0",
            clientVersion: "2.1.0",
        }),
        row("checking.example.com", "checking"),
    ];

    const FAULT_CODES = Object.keys(FaultCatalog.DEFINITIONS);
    let faultCode = $state("QUIC01");
    const fault = $derived(FaultCatalog.resolve(faultCode));
    const faultActions = $derived([
        { label: fault.primaryAction.label, onclick: noop, primary: true },
        ...(fault.secondaryAction
            ? [{ label: fault.secondaryAction.label, onclick: noop, primary: false }]
            : []),
    ]);
</script>

{#if !DEV}
    <p style="padding: 2rem; font-family: monospace">Not available in a release build.</p>
{:else}
    <div class="gallery">
        <p class="note">
            Container queries, so these are faithful. Scroll each frame — the footbar must
            stay on screen at every step.
        </p>

        {#snippet pair(title: string, body: import("svelte").Snippet)}
            <section class="row">
                <h2>{title}</h2>
                <div class="frames">
                    <figure>
                        <figcaption>phone · 412 × 892</figcaption>
                        <div class="rad-frame rad-frame--phone">
                            <div class="rad-grain"></div>
                            {@render body()}
                        </div>
                    </figure>
                    <figure>
                        <figcaption>desktop · 1280 × 800</figcaption>
                        <div class="rad-frame rad-frame--desktop">
                            <div class="rad-grain"></div>
                            {@render body()}
                        </div>
                    </figure>
                </div>
            </section>
        {/snippet}

        <div class="steps">
            Intro step:
            {#each [1, 2, 3, 4] as n (n)}
                <button class:on={introStep === n} onclick={() => (introStep = n)}>{n}</button>
            {/each}
        </div>

        {@render pair(`Intro · step ${introStep}`, introBody)}
        {#snippet introBody()}
            <IntroScreen
                step={introStep}
                onstep={(s) => (introStep = s)}
                onnext={noop}
                onback={noop}
                onskip={noop}
            />
        {/snippet}

        {@render pair("Gate", gateBody)}
        {#snippet gateBody()}
            <GateScreen onhaveserver={noop} onnoserver={noop} />
        {/snippet}

        {@render pair("Sign in · reachable", signInOkBody)}
        {#snippet signInOkBody()}
            <SignInScreen
                address="s4.bedrock-legends.bedrockvc.stream"
                verdict={VERDICTS.ok}
                appVersion="1.0.0-beta.8"
                oninput={noop}
                onconnect={noop}
                onprivacy={noop}
                onrevisit={noop}
            />
        {/snippet}

        {@render pair("Sign in · nothing there", signInBadBody)}
        {#snippet signInBadBody()}
            <SignInScreen
                address="nope.example.com"
                verdict={VERDICTS.bad}
                appVersion="1.0.0-beta.8"
                oninput={noop}
                onconnect={noop}
                onprivacy={noop}
                onrevisit={noop}
            />
        {/snippet}

        {@render pair("Sign-in code", codeBody)}
        {#snippet codeBody()}
            <CodeScreen
                server="s4.bedrock-legends.bedrockvc.stream"
                error=""
                isSubmitting={false}
                appVersion="1.0.0-beta.8"
                onsubmit={noop}
                onback={noop}
            />
        {/snippet}

        {@render pair("Nobody has set it up", noServerBody)}
        {#snippet noServerBody()}
            <NoServerScreen
                onguide={noop}
                oncopyinvite={noop}
                onwatch={noop}
                onwiki={noop}
                ondiscord={noop}
                onsignin={noop}
            />
        {/snippet}

        {@render pair("Not connected", connectErrorBody)}
        {#snippet connectErrorBody()}
            <ConnectErrorScreen
                server="s4.bedrock-legends.bedrockvc.stream"
                appVersion="1.0.0-beta.8"
                onretry={noop}
                onchangeserver={noop}
                onwiki={noop}
                ondiscord={noop}
            />
        {/snippet}

        {@render pair("Servers · every state", rosterBody)}
        {#snippet rosterBody()}
            <ServerListScreen
                entries={ROSTER}
                isRefreshing={false}
                appVersion="1.0.0-beta.8"
                onchoose={noop}
                onforget={noop}
                onadd={noop}
                onrefresh={noop}
                onsettings={noop}
            />
        {/snippet}

        {@render pair("Servers · one, re-checking", rosterOneBody)}
        {#snippet rosterOneBody()}
            <ServerListScreen
                entries={[ROSTER[0]]}
                isRefreshing={true}
                appVersion="1.0.0-beta.8"
                onchoose={noop}
                onforget={noop}
                onadd={noop}
                onrefresh={noop}
                onsettings={noop}
            />
        {/snippet}

        {@render pair("Setup · microphone", micBody)}
        {#snippet micBody()}
            <MicrophoneScreen state="idle" step={1} total={3} onrequest={noop} />
        {/snippet}

        {@render pair("Setup · notifications", notifyBody)}
        {#snippet notifyBody()}
            <NotificationsScreen state="idle" step={2} total={3} onrequest={noop} />
        {/snippet}

        {@render pair("Setup · devices", devicesBody)}
        {#snippet devicesBody()}
            <DevicesScreen
                step={3}
                total={3}
                inputLevel={0.42}
                gateOpen={true}
                ontestspeaker={async () => true}
                oncontinue={noop}
            />
        {/snippet}

        <div class="steps">
            Fault:
            {#each FAULT_CODES as code (code)}
                <button class:on={faultCode === code} onclick={() => (faultCode = code)}>
                    {code}
                </button>
            {/each}
        </div>

        {@render pair(`Error · ${fault.code} · ${fault.severity}`, faultBody)}
        {#snippet faultBody()}
            <FaultScreen
                code={fault.code}
                title={fault.title}
                message={fault.message}
                icon={fault.icon}
                severity={fault.severity}
                category={fault.category}
                chip={fault.chip}
                caption={fault.caption}
                label={fault.label}
                hint={fault.hint}
                appVersion="1.0.0-beta.8"
                actions={faultActions}
            >
                {#snippet footnote()}
                    {#if fault.severity !== "ok"}
                        <span
                            class="rad-label rad-rise"
                            style="--d: 360; display: block; margin-top: 26px"
                        >
                            Still stuck?
                        </span>
                        <div class="rad-swatchrow rad-rise" style="--d: 390">
                            <button class="rad-pill-link">
                                Wiki <span class="rad-pill-link__ext">&#8599;</span>
                            </button>
                            <button class="rad-pill-link">
                                Discord <span class="rad-pill-link__ext">&#8599;</span>
                            </button>
                        </div>
                    {/if}
                {/snippet}
            </FaultScreen>
        {/snippet}

        <!-- The one state the route cannot be talked into showing: pressing Update Now
             installs the update for real. -->
        {@render pair("Error · UPD01 · installing", updatingBody)}
        {#snippet updatingBody()}
            <FaultScreen
                code="UPD01"
                title={FaultCatalog.DEFINITIONS.UPD01.title}
                message={FaultCatalog.DEFINITIONS.UPD01.message}
                icon="download"
                severity="ok"
                category={FaultCatalog.DEFINITIONS.UPD01.category}
                chip={FaultCatalog.DEFINITIONS.UPD01.chip}
                caption={FaultCatalog.DEFINITIONS.UPD01.caption}
                label={FaultCatalog.DEFINITIONS.UPD01.label}
                hint={FaultCatalog.DEFINITIONS.UPD01.hint}
                appVersion="1.0.0-beta.8"
                actions={[{ label: "Updating…", onclick: noop, primary: true, disabled: true }]}
                working={true}
                workingPhrases={FaultCatalog.UPDATE_PHRASES}
            />
        {/snippet}

        {@render pair("Launch", launchBody)}
        {#snippet launchBody()}
            <RadScreen label="Starting up">
                <div class="launch">
                    <Loader
                        loading={true}
                        phrases={["Checking for updates…", "Almost there…"]}
                        slowAfterSeconds={0}
                    />
                </div>
                {#snippet footbar()}
                    <span class="rad-label">Bedrock Voice Chat</span>
                {/snippet}
            </RadScreen>
        {/snippet}
    </div>
{/if}

<style>
    .gallery {
        min-height: 100dvh;
        padding: 24px;
        background: #120a20;
        color: #fbf8ff;
        font-family: ui-sans-serif, system-ui, sans-serif;
    }

    .note,
    .steps {
        margin: 0 0 20px;
        font-size: 13px;
        color: #b3a4d0;
    }

    .steps button {
        margin-left: 6px;
        padding: 3px 10px;
        border: 1px solid #5b4487;
        border-radius: 6px;
        background: transparent;
        color: inherit;
        cursor: pointer;
    }

    .steps button.on {
        background: #8239d8;
        color: #fff;
    }

    .row {
        margin-bottom: 40px;
    }

    .row h2 {
        margin: 0 0 10px;
        font-size: 14px;
        letter-spacing: 0.12em;
        text-transform: uppercase;
        color: #d6cbea;
    }

    .frames {
        display: flex;
        flex-wrap: wrap;
        gap: 24px;
        align-items: flex-start;
    }

    figure {
        margin: 0;
    }

    figcaption {
        margin-bottom: 6px;
        font-family: ui-monospace, Consolas, monospace;
        font-size: 11px;
        color: #8f7fb5;
    }

    .launch {
        flex: 1 1 auto;
        min-height: 0;
        display: flex;
        align-items: center;
        justify-content: center;
    }
</style>
