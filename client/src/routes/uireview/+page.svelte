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
    import type { ResolveVerdict } from "../../js/app/login/AddressResolver";

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

        {@render pair("Launch", launchBody)}
        {#snippet launchBody()}
            <RadScreen label="Starting up">
                <div class="launch">
                    <Loader
                        loading={true}
                        withIntro={false}
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
