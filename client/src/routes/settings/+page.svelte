<script lang="ts">
    import "../../css/app.css";
    import "../../css/settings-responsive.css";
    import Settings from "../../js/app/settings.ts";
    import Sidebar from "../../components/settings/Sidebar.svelte";

    import { onMount } from "svelte";

    onMount(() => {
      window.App = new Settings();
      window.dispatchEvent(new CustomEvent("app:mounted"));
      document.querySelector("body")?.classList.add("is-sidebar-open");
      window.App.initialize();
      window.App.preloader();
    });
</script>

<div id="root" class="min-h-100vh cloak flex grow bg-slate-50 dark:bg-navy-900">
  <!-- Desktop sidebar -->
  <div id="main-sidebar-container" class="sidebar print:hidden">
    <Sidebar activePage="audio.svelte" />
  </div>

  <main class="main-content flex-1 settings-main-content 
    md:static md:block md:transform-none md:z-auto md:bg-transparent md:overflow-visible md:px-[var(--margin-x)] md:pb-8">  
    <!-- Content will be dynamically mounted here by the Sidebar component -->
  </main>
</div>
