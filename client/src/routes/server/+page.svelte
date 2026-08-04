<script lang="ts">
  import "../../css/app.css";
  import { onMount, onDestroy } from "svelte";
  import RadFrame from "../../components/shell/RadFrame.svelte";
  import RadConfirm from "../../components/shell/RadConfirm.svelte";
  import ServerListScreen from "../../components/server/ServerListScreen.svelte";
  import PreflightPanel from "../../components/server/PreflightPanel.svelte";
  import Server from "../../js/app/server.ts";
  import Analytics from "../../js/app/analytics";
  import type { NextAction } from "../../js/app/shell/NextAction";
  import type { ServerRosterEntry } from "../../js/app/server/ServerRosterEntry";

  let app: Server | null = null;
  const unsubs: Array<() => void> = [];

  let entries = $state<readonly ServerRosterEntry[]>([]);
  let isRefreshing = $state(false);

  /** Which server's readout is open, by url. Empty when the panel is closed. */
  let reading = $state("");
  /** The server a removal is being confirmed for, or null when nothing is being confirmed. */
  let forgetting = $state<ServerRosterEntry | null>(null);

  // Resolved rather than captured, so an open readout follows its own server's checks as
  // they land instead of freezing on the state it opened with.
  let openEntry = $derived(entries.find((entry) => entry.server === reading) ?? null);

  function apply(action: NextAction): void {
    if (action.kind === "navigate") window.location.href = action.href;
  }

  onMount(() => {
    const instance = new Server();
    app = instance;
    window.App = instance;
    window.dispatchEvent(new CustomEvent("app:mounted"));

    unsubs.push(instance.roster.entries.subscribe((v) => (entries = v)));
    unsubs.push(instance.roster.isRefreshing.subscribe((v) => (isRefreshing = v)));

    instance
      .initialize()
      .then((landing) => {
        // The overlay comes down only when this page is the destination. A redirect and a
        // deep-link handoff are both already navigating, and showing the list on the way
        // past would flash a screen nobody asked for.
        if (landing.kind === "navigate") window.location.href = landing.href;
        else if (landing.kind === "show") instance.showPreloader();
      })
      .catch(() => instance.showPreloader());

    document.querySelector("body")?.classList.remove("has-min-sidebar");
  });

  onDestroy(() => {
    for (const off of unsubs) off();
  });

  async function choose(server: string): Promise<void> {
    const next = await app!.roster.choose(server);
    if (next.kind === "navigate" && next.href.startsWith("/dashboard")) {
      Analytics.track("ServerSelected");
    }
    apply(next);
  }

  /**
   * The readout and the confirm are both modals, and the kit allows one at a time: a confirm
   * stacked over a readout leaves no way to tell which one a Cancel belongs to.
   */
  function askToForget(entry: ServerRosterEntry): void {
    reading = "";
    forgetting = entry;
  }

  async function confirmForget(): Promise<void> {
    const entry = forgetting;
    forgetting = null;
    if (entry) apply(await app!.roster.remove(entry.server));
  }
</script>

<RadFrame>
  <ServerListScreen
    {entries}
    {isRefreshing}
    onchoose={choose}
    onopen={(server) => (reading = server)}
    onadd={() => apply(app!.addServer())}
    onrecheckall={() => void app!.roster.refreshAll()}
  />

  <PreflightPanel
    entry={openEntry}
    onclose={() => (reading = "")}
    onrecheck={(server) => void app!.roster.recheck(server)}
    onremove={askToForget}
    onchoose={choose}
  />

  <RadConfirm
    open={forgetting !== null}
    title="Forget this server?"
    confirmLabel="Forget it"
    cancelLabel="Keep it"
    destructive={true}
    onconfirm={confirmForget}
    oncancel={() => (forgetting = null)}
  >
    {#snippet body()}
      <b>{forgetting?.host}</b> is removed from this list and its saved sign-in is cleared
      from this device. Nothing on the server changes, and you can add it again with its
      address.
    {/snippet}
  </RadConfirm>
</RadFrame>
