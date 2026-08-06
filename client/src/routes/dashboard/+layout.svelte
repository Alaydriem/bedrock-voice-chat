<script lang="ts">
  import "../../css/app.css";
  import { onDestroy, onMount, setContext, type Snippet } from "svelte";
  import { get } from "svelte/store";
  import { Store } from "@tauri-apps/plugin-store";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { error as logError, info } from "@tauri-apps/plugin-log";
  import { goto } from "$app/navigation";
  import { page } from "$app/state";
  import Cover from "../../components/shell/Cover.svelte";
  import { SettingsRoute } from "../../js/app/settings/SettingsRoute";
  import { UpdateStatus } from "../../js/app/settings/UpdateStatus";
  import { UpdatePoller } from "../../js/app/shell/UpdatePoller";
  import { UPDATE_STATUS_KEY } from "../../js/app/shell/UpdateStatusContext";
  import { BootTimeline } from "../../js/app/shell/BootTimeline";
  import type { SelfSnapshot } from "$radial/core/controllers/SelfState";
  import type { LevelSource } from "$radial/core/sources/LevelSource";
  import { ConstantLevelSource } from "$radial/core/sources/LevelSource";
  import RadFrame from "../../components/shell/RadFrame.svelte";
  import DashboardScreen from "../../components/dashboard/DashboardScreen.svelte";
  import GroupsPanel from "../../components/dashboard/GroupsPanel.svelte";
  import NearbyRing from "../../components/dashboard/NearbyRing.svelte";
  import Roster from "../../components/dashboard/Roster.svelte";
  import StatusPanel from "../../components/dashboard/StatusPanel.svelte";
  import Notification from "../../components/events/Notification.svelte";
  import Dashboard from "../../js/app/dashboard.ts";
  import { PlayerPresenceManager } from "../../js/app/components/dashboard/presence.ts";
  import Analytics from "../../js/app/analytics";
  import { DiagnosticsManager, type LinkHealth } from "../../js/app/dashboard/DiagnosticsManager";
  import { GroupsView } from "../../js/app/dashboard/GroupsView";
  import type { GroupRowView } from "../../js/app/dashboard/GroupRowView";
  import type { NearbyPlayer } from "../../js/app/dashboard/NearbyPlayer";
  import type { RailServer } from "../../js/app/dashboard/RailView";
  import { RosterHandoff } from "../../js/app/dashboard/RosterHandoff";
  import { RosterView } from "../../js/app/dashboard/RosterView";
  import GameNameUtils from "../../js/app/utils/GameNameUtils";

  interface Props {
    children?: Snippet;
  }
  let { children }: Props = $props();

  /**
   * Settings is a child route, so it is on screen exactly when the URL says so.
   *
   * Deriving this from the path rather than holding a flag is what makes the back
   * gesture, a deep link and the rail's gear all do the same thing without any of them
   * knowing about the others.
   */
  const coverOpen = $derived(page.url.pathname.startsWith(SettingsRoute.base));

  let app = $state<Dashboard | null>(null);
  let presence: PlayerPresenceManager | undefined;
  const diagnostics = new DiagnosticsManager();
  const groupsView = new GroupsView();
  const unsubs: Array<() => void> = [];

  /**
   * One per session, so the nav badge and the About row read the same object and the check does
   * not restart every time settings is opened. Published rather than passed: the settings
   * screen is a descendant route, and a layout cannot hand props to a page.
   */
  const updates = new UpdateStatus();
  const updatePoller = new UpdatePoller(updates);
  setContext(UPDATE_STATUS_KEY, updates);

  let servers = $state<readonly RailServer[]>([]);
  let player = $state("");
  /**
   * The same client, as `game:gamertag`.
   *
   * Held beside `player` rather than instead of it: the top bar shows a gamertag, and every
   * comparison against something the server sent needs the form the certificate carried.
   */
  let identity = $state("");
  let currentHost = $state("");
  let ready = $state(false);
  let scope = $state(120);

  let inEarshot = $state<readonly NearbyPlayer[]>([]);
  let approaching = $state<readonly NearbyPlayer[]>([]);
  let groupRows = $state<readonly GroupRowView[]>([]);
  let joinedId = $state<string | null>(null);
  let now = $state(Date.now());

  /** The one avatar expanded for adjustment, by CN name. */
  let opened = $state<string | null>(null);
  let statusOpen = $state(false);
  let reconnecting = $state(false);

  let snapshot = $state<import("../../js/bindings/LinkDiagnosticsSnapshot").LinkDiagnosticsSnapshot | null>(null);
  let voice = $state<import("../../js/app/dashboard/SelfController").VoiceDiagnostics | null>(null);
  let health = $state<LinkHealth>({ connected: true, reconnecting: false });

  /**
   * The roster, gated on the link.
   *
   * A card that survives a dead link asserts that person can hear you. That is the misleading
   * kind of wrong — it keeps somebody talking into nothing — so the screen falls back to the ring
   * and says so. Gated here rather than by clearing the managers, so everything is still there
   * the moment the link returns and nothing has to be re-fetched.
   */
  const linkUp = $derived(health.connected);
  const nearby = $derived<readonly NearbyPlayer[]>(linkUp ? inEarshot : []);
  const nearing = $derived<readonly NearbyPlayer[]>(linkUp ? approaching : []);

  let selfState = $state<SelfSnapshot>({
    muted: false,
    deafened: false,
    recording: false,
    mode: "activated",
    holding: false,
    transmitting: true,
  });

  const groupName = $derived(groupRows.find((g) => g.joined)?.name ?? "");
  const inGroup = $derived(groupRows.find((g) => g.joined));

  /**
   * Group members as roster entries.
   *
   * A channel member is at full volume whatever the distance, which is what a channel is for,
   * so they are shown without one rather than with a number that does not govern anything.
   */
  const groupRoster = $derived<readonly NearbyPlayer[]>(
    (linkUp ? (inGroup?.members ?? []) : [])
      .filter((member) => member.name !== identity)
      .map((member) => ({
      name: member.name,
      gamertag: member.gamertag,
      game: "minecraft",
      hue: "var(--color-rad-brand-lift)",
      presence: "voice" as const,
      distance: 0,
      bearing: 0,
      elevation: 0,
      inEarshot: true,
    })),
  );

  /**
   * Earshot, with the group taken out of it.
   *
   * Somebody in your group who is also standing next to you satisfied both lists and got a card in
   * each — the same person twice, once at full volume and once with a distance beside them. The
   * distance was the misleading half: a channel routes their audio whatever it says, so the number
   * described nothing that was happening.
   *
   * The group wins because it is the stronger statement. Compared on the canonical identity,
   * which both lists now carry: channel membership from the certificate's Common Name, the
   * position feed from the same composition. On the bare gamertag this would also have hidden
   * `hytale:Bob` from earshot because `minecraft:Bob` was in the group.
   */
  const groupTags = $derived(new Set(groupRoster.map((member) => member.name)));
  const earshot = $derived<readonly NearbyPlayer[]>(
    nearby.filter((person) => !groupTags.has(person.name)),
  );

  // Counted off the list it names, so the number in the top bar and the number on the section
  // rule cannot disagree about the same room.
  const headline = $derived(RosterView.headline(earshot.length, nearing.length));

  const silent = new ConstantLevelSource(0);
  function sourceFor(name: string): LevelSource {
    return app?.levels?.for(name) ?? silent;
  }

  let nearbyRing = $state<ReturnType<typeof NearbyRing> | null>(null);
  let rosterEl = $state<HTMLElement | null>(null);

  /** Cards waiting for their flyer. Held back so the mark reads as becoming the card. */
  let pending = $state<ReadonlySet<string>>(new Set());

  const handoff = new RosterHandoff((player) => nearbyRing?.pointFor(player) ?? null);

  /** Everyone currently holding a card, which is what flies in either direction. */
  const carded = $derived<readonly NearbyPlayer[]>([...groupRoster, ...earshot]);

  /**
   * The flight between the ring and the roster.
   *
   * Two halves, because the two directions need opposite moments. Departures have to be measured
   * before Svelte removes the cards — a detached element measures as 0x0 at the origin, which
   * would fling every flyer out of the top-left corner — so `$effect.pre` reads them while they
   * are still on screen. Arrivals need the card to exist first, so the plain `$effect` does those
   * after the DOM has settled.
   */
  $effect.pre(() => {
    handoff.capture(rosterEl, carded.length > 0);
  });

  $effect(() => {
    const flying = handoff.settle(rosterEl, carded, (name) => {
      const next = new Set(pending);
      next.delete(name);
      pending = next;
    });
    if (flying) pending = flying;
  });

  /**
   * Per-player gain and mute, held reactively.
   *
   * Reading these through `playerManager.get()` inside the template was a plain function call
   * with no dependency behind it, so pressing mute changed the store and re-rendered nothing —
   * the button never turned red because the card never re-ran.
   */
  let settings = $state<Map<string, { gain: number; muted: boolean }>>(new Map());

  function gainFor(name: string): number {
    return settings.get(GameNameUtils.canonical(name))?.gain ?? 1;
  }

  function mutedFor(name: string): boolean {
    return settings.get(GameNameUtils.canonical(name))?.muted ?? false;
  }

  onMount(() => {
    const instance = new Dashboard();
    app = instance;
    window.App = instance;
    window.dispatchEvent(new CustomEvent("app:mounted"));

    const clock = setInterval(() => (now = Date.now()), 1_000);
    unsubs.push(() => clearInterval(clock));

    // Outside the initialize chain: a dashboard that could not connect should still learn that
    // a newer build exists, which may be the reason it could not.
    updatePoller.start();
    unsubs.push(() => updatePoller.stop());

    instance
      .initialize()
      .then(async (landing) => {
        // The overlay stays up over a redirect. Lifting it there shows a dashboard already on
        // its way to the sign-in or an error page.
        if (landing.kind === "navigate") {
          BootTimeline.shared().mark(`REDIRECTED: ${landing.href}`);
          BootTimeline.shared().report();
          window.location.href = landing.href;
          return;
        }

        servers = instance.rail;
        player = instance.gamertag;
        identity = instance.identity;
        currentHost = instance.host();
        scope = instance.feedScope;

        if (instance.selfController) {
          unsubs.push(instance.selfController.state.subscribe((s) => (selfState = s)));
          unsubs.push(instance.selfController.diagnostics.subscribe((v) => (voice = v)));
        }
        if (instance.playerManager) {
          unsubs.push(
            instance.playerManager.playersMap.subscribe((map) => {
              const next = new Map<string, { gain: number; muted: boolean }>();
              for (const [key, player] of map) next.set(key, { ...player.settings });
              settings = next;
            }),
          );
        }
        if (instance.nearby) {
          unsubs.push(instance.nearby.inEarshot.subscribe((v) => (inEarshot = v)));
          unsubs.push(instance.nearby.approaching.subscribe((v) => (approaching = v)));
        }

        ready = true;
        instance.showPreloader();
        BootTimeline.shared().mark("OVERLAY DISMISSED");
        BootTimeline.shared().report();

        await diagnostics.start();
        unsubs.push(diagnostics.snapshot.subscribe((v) => (snapshot = v)));
        unsubs.push(diagnostics.health.subscribe((v) => (health = v)));

        /**
         * Automatic recovery, which had no listener at all.
         *
         * The backend probes a lost server and emits `trigger_refresh` the moment it answers
         * again. The only handler for it lived on the old dashboard's sidebar — a component this
         * screen does not mount — so on this screen the event went nowhere: the server came back,
         * the client was told, and it sat on "Trying to reach the server" until somebody
         * intervened. That is the "it never reconnected" case.
         */
        try {
          const webview = getCurrentWebviewWindow();
          unsubs.push(
            await webview.listen("trigger_refresh", () => {
              info("Dashboard: server is reachable again, reconnecting");
              void reconnect();
            }),
          );
        } catch (e) {
          logError(`Dashboard: could not listen for a recovery signal: ${e}`);
        }

        /**
         * Come up connected, or make it so.
         *
         * `initialize` decides whether to build the session from whether the *audio streams* are
         * stopped, and they are not: the Rust side outlives a webview reload, so a reload takes
         * the warm path and rebuilds nothing. That is right for audio and says nothing about the
         * voice link, which can be gone — after which the position feed reconnects on its own
         * ticket and fills the roster with people who cannot hear you.
         *
         * The seeded health is the evidence, since the backend reports no diagnostics at all
         * while disconnected.
         */
        if (!get(diagnostics.health).connected) {
          info("Dashboard: mounted over a dead link, reconnecting");
          void reconnect();
        }

        groupsView.start();
        unsubs.push(() => groupsView.stop());
        await startGroups(instance);

        // Audio-driven presence stays wired as the roster's fallback: if the position feed is
        // unreachable, cards still appear for whoever can be heard, just without a distance.
        try {
          const store = await Store.load("store.json", { autoSave: false, defaults: {} });
          if (instance.playerManager) {
            presence = new PlayerPresenceManager(store, instance.playerManager);
            await presence.initialize();
          }
        } catch (e) {
          logError(`Dashboard: could not start presence: ${e}`);
        }
      })
      .catch(() => instance.showPreloader());
  });

  /** The server's channels, mirrored so the rows can be rebuilt from a rune. */
  let channelList = $state<readonly import("../../js/bindings/Channel").Channel[]>([]);

  async function startGroups(instance: Dashboard): Promise<void> {
    const channels = instance.channelManager;
    if (!channels) return;
    try {
      // Loads the list and recovers membership. Without it the pane rendered nothing at all:
      // `channel_event` carries changes, and channels that existed before this page loaded
      // never announce themselves.
      await channels.initialize();
      await channels.startListening();
    } catch (e) {
      logError(`Dashboard: could not start channels: ${e}`);
    }

    unsubs.push(
      channels.currentUserChannelId.subscribe((id) => {
        joinedId = id;
        // A join or leave is the only activity signal available for a group you are not in, so
        // it is what makes the row stir.
        if (id) groupsView.stir(id);
      }),
    );
    unsubs.push(channels.channels.subscribe((list) => (channelList = list)));
  }

  /**
   * Who is audible, as a value that only changes when the answer does.
   *
   * Depending on `nearby` directly would rebuild every row twice a second — snapshots arrive at
   * 2 Hz and each one is a fresh array — to express a set that only changes when somebody walks
   * in or out. A string compares by value, so this stops propagating while membership holds.
   *
   * Joined on a newline rather than a space: Xbox gamertags contain spaces, and splitting on one
   * would turn "Some Gamer" into two names that match nobody.
   */
  const audibleKey = $derived(
    [...new Set(nearby.map((p) => p.name))].sort().join("\n"),
  );

  /**
   * One rows subscription, rebuilt whenever any of its inputs move.
   *
   * `rows` is a snapshot over values rather than a live view of them, so binding it once to the
   * channel list left the others frozen: joining a group you were already listed in moved
   * `joinedId` and nothing else, so the row went on offering Join; and the can-you-hear-them dot
   * beside each member stayed at whatever earshot was when the list last changed.
   *
   * The effect's cleanup releases the previous subscription before the next is made. Pushing
   * each new one onto the teardown list instead freed nothing until the page was destroyed, so
   * ordinary activity left a growing crowd of stores writing to one variable.
   */
  $effect(() => {
    const audible = new Set(audibleKey === "" ? [] : audibleKey.split("\n"));
    return groupsView
      .rows(channelList, joinedId, audible, identity)
      .subscribe((v) => (groupRows = v));
  });

  /** The group whose editor is open. Set on create so the new one opens ready to be named. */
  let editId = $state<string | null>(null);

  /**
   * Create, join, and open for editing.
   *
   * Creating a group you are not in is never what was meant — the reason to make one is to talk
   * in it. And "New group" is a placeholder, not a name, so the editor opens on it rather than
   * leaving the user to discover that renaming is behind a swipe.
   */
  async function createGroup(): Promise<void> {
    const channels = app?.channelManager;
    if (!channels) return;
    const id = await channels.createChannel("New group");
    if (!id) return;
    await channels.joinChannel(id, identity);
    editId = id;
  }

  function leaveGroup(id: string): void {
    void app?.channelManager?.leaveChannel(id, identity);
  }

  /** Closing is deleting. Only the creator can, and the server is the one that enforces it. */
  function closeGroup(id: string): void {
    void app?.channelManager?.deleteChannel(id);
  }

  function renameGroup(id: string, name: string): void {
    void app?.channelManager?.renameChannel(id, name);
  }

  onDestroy(() => {
    for (const off of unsubs) off();
    presence?.cleanup();
    diagnostics.stop();
  });

  function signOut(): void {
    Analytics.track("Logout");
    void app?.signOut().then((next) => {
      if (next.kind === "navigate") window.location.href = next.href;
    });
  }

  function onmute(name: string, muted: boolean): void {
    void app?.playerManager?.updatePlayerMute(name, muted);
  }

  function ongain(name: string, gain: number): void {
    void app?.playerManager?.updatePlayerGain(name, gain);
  }

  function joinGroup(id: string): void {
    const channels = app?.channelManager;
    if (!channels) return;
    if (joinedId === id) void channels.leaveChannel(id, identity);
    else void channels.joinChannel(id, identity);
  }

  /**
   * Reconnect without leaving the panel.
   *
   * `Dashboard.changeNetworkStream` routes its failures to an error page, which is right for
   * boot and wrong for a button inside a status readout: a retry that fails should say so
   * where it was pressed.
   */
  async function reconnect(): Promise<void> {
    if (reconnecting || !app) return;
    reconnecting = true;
    try {
      await app.reconnect();
    } finally {
      reconnecting = false;
    }
  }

  async function copyReport(): Promise<void> {
    const report = await diagnostics.report();
    if (report) await navigator.clipboard.writeText(report).catch(() => {});
  }
</script>

<RadFrame>
  <Cover open={coverOpen} ondismiss={() => void goto("/dashboard")}>
    {#snippet under()}
  {#if ready && app?.selfController}
    <DashboardScreen
      {servers}
      serverName={currentHost}
      {currentHost}
      {player}
      self={app.selfController}
      {selfState}
      {headline}
      {groupName}
      {statusOpen}
      onswitch={(server) => (window.location.href = `/dashboard?server=${encodeURIComponent(server)}`)}
      onadd={() => (window.location.href = "/")}
      onsettings={() => void goto(SettingsRoute.href("audio"))}
      onsignout={signOut}
      onstatus={(open) => (statusOpen = open)}
    >
      {#snippet groups()}
        <GroupsPanel
          groups={groupRows}
          {now}
          onjoin={joinGroup}
          oncreate={() => void createGroup()}
          {editId}
          onedit={(id) => (editId = id)}
          onleave={leaveGroup}
          onclosegroup={closeGroup}
          onrename={renameGroup}
        />
      {/snippet}

      {#snippet main()}
        <!-- The ring is the empty state and the approach. Once anybody is close enough to
             hear, the roster owns the stage and the ring gets out of the way. -->
        <NearbyRing
          bind:this={nearbyRing}
          approaching={nearing}
          {scope}
          gone={carded.length > 0}
          connected={linkUp}
          reconnecting={health.reconnecting}
          onstatus={() => (statusOpen = true)}
        />

        {#if carded.length}
          <div class="rad-roster" bind:this={rosterEl}>
            <!-- The group leads. Joining a channel is deliberate and proximity is ambient, so
                 the people you went out of your way to talk to are never below a list of
                 neighbours. -->
            {#if inGroup}
              <Roster
                title={inGroup.name}
                players={groupRoster}
                inGroup={true}
                {sourceFor}
                {gainFor}
                {mutedFor}
                {onmute}
                {ongain}
                {opened}
                onopen={(name) => (opened = name)}
                {pending}
              >
                {#snippet action()}
                  <button class="rad-leave-btn" onclick={() => joinGroup(inGroup.id)}>
                    Leave
                  </button>
                {/snippet}
              </Roster>
            {/if}

            <Roster
              title="In earshot"
              players={earshot}
              {sourceFor}
              {gainFor}
              {mutedFor}
              {onmute}
              {ongain}
              {opened}
              onopen={(name) => (opened = name)}
              {pending}
            />
          </div>
        {/if}

        <StatusPanel
            {snapshot}
            {health}
            {voice}
            selfMode={selfState.mode}
            pttIdle={selfState.mode === "ptt" && !selfState.holding}
            visiblePlayers={carded.length}
            {reconnecting}
            onreconnect={reconnect}
            oncopy={copyReport}
            onreset={() => void diagnostics.reset()}
            onclose={() => (statusOpen = false)}
          />
      {/snippet}
    </DashboardScreen>
  {/if}
    {/snippet}

    {@render children?.()}
  </Cover>
</RadFrame>

<Notification />
