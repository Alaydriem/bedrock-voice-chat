<script lang="ts">
  import "../../css/app.css";
  import Server from "../../js/app/server.ts";
  import ServerSelectTopBar from "../../components/ServerSelectTopBar.svelte";
  import { onMount } from 'svelte';

  let isRefreshing = $state(false);
  let appInstance: Server | null = null;

  onMount(() => {
    appInstance = new Server();
    window.App = appInstance;
    window.dispatchEvent(new CustomEvent("app:mounted"));

    appInstance.initialize();
    appInstance.preloader();
    document.querySelector("body")?.classList.remove("has-min-sidebar");
  });

  async function handleRefreshAll() {
    if (!appInstance || isRefreshing) return;
    isRefreshing = true;
    try {
      await appInstance.renderServerList();
    } finally {
      isRefreshing = false;
    }
  }

  function handleAddServer() {
    window.location.href = "/login?addserver=true&return=/server";
  }
</script>

<div id="root" class="min-h-100vh flex grow">
  <main class="w-full px-[var(--margin-x)] pb-8 mt-8 pt-8">
    <div class="mb-8">
      <ServerSelectTopBar
        isRefreshing={isRefreshing}
        onRefreshAll={handleRefreshAll}
        onAddServer={handleAddServer}
      />
    </div>
    <div id="server-avatar-container" class="grid grid-cols-1 gap-5 sm:grid-cols-2 sm:gap-6 xl:grid-cols-3">

    </div>
  </main>
</div>
