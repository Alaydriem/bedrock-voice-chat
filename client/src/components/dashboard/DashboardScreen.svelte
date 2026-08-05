<script lang="ts">
    import { onDestroy, onMount } from "svelte";
    import Icon from "$radial/components/Icon.svelte";
    import ServerGlyph from "$radial/components/ServerGlyph.svelte";
    import { Handoff } from "$radial/core/controllers/Handoff";
    import { MENU_DIVIDER, Menu, type MenuEntry } from "$radial/core/controllers/Menu";
    import { Sheet } from "$radial/core/controllers/Sheet";
    import type { SelfSnapshot } from "$radial/core/controllers/SelfState";
    import type { RailServer } from "../../js/app/dashboard/RailView";
    import type { SelfController } from "../../js/app/dashboard/SelfController";
    import SelfControls from "./SelfControls.svelte";
    import ServerRail from "./ServerRail.svelte";

    interface Props {
        servers: readonly RailServer[];
        serverName: string;
        currentHost: string;
        player: string;
        self: SelfController;
        selfState: SelfSnapshot;
        headline: string;
        groupName?: string;
        statusOpen?: boolean;
        onswitch: (server: string) => void;
        onadd: () => void;
        onsettings: () => void;
        onsignout: () => void;
        onstatus: (open: boolean) => void;
        /** The stage's body — the ring, the roster, the status panel. */
        main?: import("svelte").Snippet;
        /** The groups pane beside the rail, repeated inside the phone sheet. */
        groups?: import("svelte").Snippet;
    }
    let {
        servers,
        serverName,
        currentHost,
        player,
        self,
        selfState,
        headline,
        groupName = "",
        statusOpen = false,
        onswitch,
        onadd,
        onsettings,
        onsignout,
        onstatus,
        main,
        groups,
    }: Props = $props();

    let shell: HTMLElement;
    let menuHost: HTMLElement;
    let menu: Menu | null = null;
    let sheet: Sheet | null = null;
    let frame = $state<HTMLElement | null>(null);

    /** Which of the two phone views is showing. Desktop shows both at once. */
    let tab = $state<"roster" | "chat">("roster");

    onMount(() => {
        // The document element, not the frame. Every `--text-rad-*` token is declared on `:root`
        // as a multiple of `--rad-type-scale`, and a custom property resolves the `var()`s inside
        // it at the element that declares it — so the scale has to be overridden on the same
        // element the tokens are declared on. Set on a descendant it would do nothing.
        document.documentElement.setAttribute("data-rad-dash", "");

        // The frame, not this component's root: it is the container-query root, the positioning
        // ancestor every overlay measures against, and the element the kit's own state classes
        // live on.
        frame = shell.closest<HTMLElement>(".rad-frame");
        if (!frame) return;
        menu = new Menu(menuHost, frame);
        sheet = new Sheet(frame);
    });

    onDestroy(() => {
        menu?.destroy();
        sheet?.destroy();
        document.documentElement.removeAttribute("data-rad-dash");
        for (const name of ["is-muted", "is-deafened", "is-status", "is-chat"]) {
            frame?.classList.remove(name);
        }
    });

    /**
     * Self state, drawn on the frame rather than on a button.
     *
     * A muted mic is a property of the session, not of the control that set it, so the kit puts
     * a coloured stripe across the top of the stage — coral for muted, amber for deafened — and
     * one glance anywhere on the screen answers "can they hear me". Toggling only the button's
     * icon leaves that stripe permanently absent, which is a state nobody can read from a
     * 32-pixel glyph.
     */
    $effect(() => {
        if (!frame) return;
        frame.classList.toggle("is-muted", selfState.muted && !selfState.deafened);
        frame.classList.toggle("is-deafened", selfState.deafened);
    });

    /** `.rad-status` is `opacity: 0` until the frame says otherwise. */
    $effect(() => {
        frame?.classList.toggle("is-status", statusOpen);
    });

    $effect(() => {
        frame?.classList.toggle("is-chat", tab === "chat");
    });

    /**
     * The burst at the button that was pressed.
     *
     * Read before the press, because the state change re-renders the pill and the element this
     * handler was handed stops existing.
     */
    function burst(event: MouseEvent, hue: string): void {
        const target = event.currentTarget;
        if (target instanceof Element) Handoff.burstAt(Handoff.centreOf(target), hue);
    }

    function pressMute(event: MouseEvent): void {
        burst(event, selfState.muted ? "#5ce383" : "#ff8266");
        self.pressMute();
    }

    function pressDeafen(event: MouseEvent): void {
        burst(event, selfState.deafened ? "#5ce383" : "#ffcf4d");
        self.pressDeafen();
    }

    /**
     * The session actions, defined once.
     *
     * Three surfaces reach the same set — the chevron beside your name, the server glyph in the
     * corner, and the slide-up sheet — and they were three separate lists. The chevron offered
     * "Switch server" and no status; the sheet offered status and no identity. Nobody can learn a
     * menu that changes depending on which edge of the screen they came in from.
     */
    type SessionAction = "add" | "settings" | "status" | "signout";

    const ACTIONS: readonly {
        action: SessionAction;
        label: string;
        icon: import("$radial/core/icons/Icons").IconName;
        danger?: boolean;
    }[] = [
            { action: "add", label: "Add a server", icon: "plus" },
            { action: "settings", label: "Settings", icon: "gear" },
            { action: "status", label: "Connection status", icon: "field" },
            { action: "signout", label: "Sign out", icon: "close", danger: true },
        ];

    function run(action: SessionAction): void {
        sheet?.close();
        if (action === "add") onadd();
        else if (action === "settings") onsettings();
        else if (action === "status") onstatus(true);
        else onsignout();
    }

    function switchTo(server: string): void {
        sheet?.close();
        onswitch(server);
    }

    /** The breakpoint the kit's own container queries use, asked of the element they measure. */
    function isPhone(): boolean {
        return (frame?.clientWidth ?? 0) <= 560;
    }

    /**
     * The chevron and the corner glyph, arriving at the same place.
     *
     * A phone gets the sheet whichever one was pressed: a dropdown anchored to a control sitting
     * at the bottom of the screen flips upward and lands under the thumb that opened it.
     */
    function openSession(): void {
        if (isPhone()) {
            sheet?.open("servers");
            return;
        }
        // Scoped to the pill. Both the pill and the phone capsule render a `.rad-self__id`, and
        // the capsule comes first in document order, so an unscoped lookup anchored the desktop
        // menu to a `display: none` element — a zero rect, which clamps it to the frame corner.
        const anchor = frame?.querySelector<HTMLElement>(".rad-self-pill .rad-self__id");
        if (!anchor) return;

        const items: MenuEntry[] = [
            { label: player, hint: "signed in" },
            MENU_DIVIDER,
            ...servers.map((server) => ({
                label: server.host,
                hint: server.isCurrent ? "current" : `as ${server.player}`,
                on: server.isCurrent,
                value: { kind: "server" as const, server: server.server },
            })),
            MENU_DIVIDER,
            ...ACTIONS.filter((entry) => !entry.danger).map((entry) => ({
                label: entry.label,
                value: { kind: "action" as const, action: entry.action },
            })),
            MENU_DIVIDER,
            ...ACTIONS.filter((entry) => entry.danger).map((entry) => ({
                label: entry.label,
                danger: true,
                value: { kind: "action" as const, action: entry.action },
            })),
        ];

        menu?.open(anchor, items, (item) => {
            const picked = item.value as
                | { kind: "server"; server: string }
                | { kind: "action"; action: SessionAction }
                | undefined;
            if (!picked) return;
            if (picked.kind === "server") switchTo(picked.server);
            else run(picked.action);
        });
    }
</script>

<!--
  These are the frame's own children, matching `examples/dashboard.html` child for child, and
  deliberately not wrapped in a `.rad-screen`.

  The login and server pages are screens; this one is the frame's whole contents. Wrapping it
  made every absolutely-positioned overlay measure against a flex container that clips
  horizontally and not vertically — so the sheets, which sit at `bottom: 0` under a
  `translateY(102%)`, hung below the fold as scrollable content instead of waiting off-screen.
-->
<div class="rad-shell" bind:this={shell}>
    <ServerRail {servers} {onswitch} {onadd} {onsettings} />

    {#if groups}
        <div class="rad-panel">
            <div class="rad-panel__head">Groups</div>
            <div class="rad-panel__body">{@render groups()}</div>
        </div>
    {/if}

    <div class="rad-stage">
        <div class="rad-dash-top">
            <span class="rad-dash-top__who">
                <span class="rad-desk-only">
                    <span class="rad-dash-top__server">{serverName}</span>
                </span>
                <button
                    class="rad-header-btn rad-phone-only"
                    data-rad-sheet-open="groups"
                    aria-label="Groups"
                >
                    <Icon name="people" />
                </button>
            </span>

            <span class="rad-dash-top__state">
                <span class="rad-desk-only">
                    <span style="display: flex; align-items: center; gap: 10px">
                        <span class="rad-health-dot"></span>
                        <span>{headline}</span>
                    </span>
                    <button
                        class="rad-header-btn"
                        class:is-on={statusOpen}
                        aria-pressed={statusOpen}
                        aria-label="Show status"
                        onclick={() => onstatus(!statusOpen)}
                    >
                        <Icon name="field" />
                    </button>
                </span>
                <button
                    class="rad-header-glyph rad-phone-only"
                    data-rad-sheet-open="servers"
                    aria-label="Servers and settings"
                >
                    <ServerGlyph name={currentHost} size={34} />
                </button>
            </span>
        </div>

        <!-- Phone: the roster and the chat are peer views rather than a stack. -->
        <div class="rad-tabs">
            <button class:is-on={tab === "roster"} onclick={() => (tab = "roster")}>
                In earshot
            </button>
            <button class:is-on={tab === "chat"} onclick={() => (tab = "chat")}>Chat</button>
        </div>

        <div class="rad-main">{@render main?.()}</div>

        <!--
          The chat dock, present and inert.

          Relaying game chat needs a surface on both sides that does not exist yet, so this is
          the affordance without the feature: a tab and a bar, so the gap is visible on the
          screen it belongs to rather than only in a document.
        -->
        <div class="rad-chat-dock">
            <div class="rad-chat-history">
                <div class="rad-chat__head">
                    <span class="rad-label">Server chat</span>
                    <span class="rad-status-chip">Not connected</span>
                    <span class="rad-spacer"></span>
                </div>
                <div class="rad-chat__body">
                    <p class="rad-roster__empty">
                        Game chat is not relayed yet. When it is, messages appear here and
                        anything typed below goes into the server's chat.
                    </p>
                </div>
            </div>
            <div class="rad-chat-bar">
                <button class="rad-chat-toggle" aria-label="Chat" disabled>
                    <Icon name="chat" />
                </button>
                <input
                    class="rad-chat-input"
                    placeholder="Message the server…"
                    aria-label="Message the server"
                    disabled
                />
                <button class="rad-chat-send" aria-label="Send" disabled>
                    <Icon name="send" />
                </button>
            </div>
        </div>

        <!-- Phone: the capsule lives in the stage, where a thumb is. -->
        <div class="rad-self-bar">
            <SelfControls
                controller={self}
                {selfState}
                name={player}
                {groupName}
                capsule={true}
                onmute={pressMute}
                ondeafen={pressDeafen}
                onidentity={openSession}
            />
        </div>
    </div>
</div>

<!-- Desktop: the pill floats over the frame, positioned against it. -->
<SelfControls
    controller={self}
    {selfState}
    name={player}
    {groupName}
    onmute={pressMute}
    ondeafen={pressDeafen}
    onidentity={openSession}
/>

<div class="rad-menu" bind:this={menuHost}></div>
<div class="rad-scrim" data-rad-sheet-scrim></div>

<div class="rad-sheet" data-rad-sheet="servers">
    <span class="rad-sheet__handle"></span>
    <h4 class="rad-sheet__title">Servers</h4>
    {#each servers as server (server.server)}
        <button
            class="rad-sheet-row"
            class:is-on={server.isCurrent}
            onclick={() => switchTo(server.server)}
        >
            <ServerGlyph name={server.host} size={30} />
            <span class="rad-sheet-row__text">
                <span class="rad-sheet-row__name">{server.host}</span>
                <span class="rad-sheet-row__host">signed in as {server.player}</span>
            </span>
            {#if server.isCurrent}
                <span class="rad-sheet-row__tick"><Icon name="check" /></span>
            {/if}
        </button>
    {/each}
    {#each ACTIONS as entry, i (entry.action)}
        <!-- Ruled off from the servers above it, and again before the one red row. -->
        {#if i === 0 || entry.danger}
            <div class="rad-sheet__divider"></div>
        {/if}
        <button
            class="rad-list-row"
            class:rad-list-row--danger={entry.danger}
            onclick={() => run(entry.action)}
        >
            <span class="rad-list-row__icon"><Icon name={entry.icon} /></span>
            {entry.label}
        </button>
    {/each}
</div>

<!--
  The phone's route into groups. Its absence is why the groups button opened nothing.

  Full height rather than parked at the bottom: the panel grows when a group is being
  renamed, and a sheet anchored to the bottom answers that by pushing everything above it
  upward — the row being edited moves while it is being read.
-->
<div class="rad-sheet rad-sheet--full" data-rad-sheet="groups">
    <span class="rad-sheet__handle"></span>
    <h4 class="rad-sheet__title">Groups</h4>
    <div class="rad-sheet__body">
        {#if groups}{@render groups()}{/if}
    </div>
</div>
