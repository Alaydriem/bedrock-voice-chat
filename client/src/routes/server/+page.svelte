<script lang="ts">
  import "../../css/app.css";
  import Server from "../../js/app/server.ts";
  import ServerSelectTopBar from "../../components/ServerSelectTopBar.svelte";
  import ServerCard from "../../components/ServerCard.svelte";
  import { onMount, onDestroy } from 'svelte';
  // @ts-ignore
  import murmurHash3 from "murmurhash3js";
  import type { ServerListEntry } from "../../js/bindings/ServerListEntry";

  let appInstance: Server | null = $state(null);
  let isRefreshing = $state(false);
  let servers: ServerListEntry[] = $state([]);

  let unsubRefreshing: (() => void) | null = null;
  let unsubServers: (() => void) | null = null;

  onMount(() => {
    const instance = new Server();
    appInstance = instance;
    window.App = instance;
    window.dispatchEvent(new CustomEvent("app:mounted"));

    unsubRefreshing = instance.isRefreshing.subscribe((value) => {
      isRefreshing = value;
    });
    unsubServers = instance.servers.subscribe((value) => {
      servers = value;
    });

    instance.initialize();
    instance.showPreloader();
    document.querySelector("body")?.classList.remove("has-min-sidebar");
  });

  onDestroy(() => {
    unsubRefreshing?.();
    unsubServers?.();
  });

  function hashServerId(server: string): string {
    const bytes = new TextEncoder().encode(server);
    const byteString = Array.from(bytes)
      .map((byte) => String.fromCharCode(byte))
      .join('');
    return murmurHash3.x86.hash128(byteString);
  }
</script>

<div id="root" class="min-h-100vh flex grow">
  <main class="w-full px-[var(--margin-x)] pb-8 mt-8 pt-8">
    <div class="mb-8">
      <ServerSelectTopBar
        isRefreshing={isRefreshing}
        onRefreshAll={() => appInstance?.refreshAll()}
        onAddServer={() => appInstance?.addServer()}
      />
    </div>
    <div class="grid grid-cols-1 gap-5 sm:grid-cols-2 sm:gap-6 xl:grid-cols-3">
      {#each servers as entry (entry.server)}
        <ServerCard
          id={hashServerId(entry.server)}
          server={entry.server}
          onRemoved={() => appInstance?.refreshAll()}
        />
      {/each}
    </div>
  </main>
</div>
