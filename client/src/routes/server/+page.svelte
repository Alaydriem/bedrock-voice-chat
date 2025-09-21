<script lang="ts">
  import "../../css/app.css";
  import Server from "../../js/app/server.ts";
  import { onMount } from 'svelte';

  onMount(() => {
    window.App = new Server();
    window.dispatchEvent(new CustomEvent("app:mounted"));

    window.App.initialize();
    window.App.preloader();
    document.querySelector("body")?.classList.remove("has-min-sidebar");
  });    
</script>

<div id="root" class="min-h-100vh flex grow">
  <main class="w-full px-[var(--margin-x)] pb-8 mt-8 pt-8">
    <div id="server-avatar-container" class="grid grid-cols-2 gap-4 sm:grid-cols-2 sm:gap-5 lg:grid-cols-3 lg:gap-6 xl:grid-cols-4">

    </div>
  </main>
</div>

