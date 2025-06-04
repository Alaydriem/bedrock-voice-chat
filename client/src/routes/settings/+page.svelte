<script lang="ts">
    import "../../css/app.css";
    import Settings from "../../js/app/settings.ts";

    import { onMount } from "svelte";

    onMount(() => {
      window.App = new Settings();
      window.dispatchEvent(new CustomEvent("app:mounted"));
      
      window.App.initialize();
    });
</script>
<div id="root" class="min-h-100vh cloak flex grow bg-slate-50 dark:bg-navy-900">
  <div id="main-sidebar-container" class="sidebar print:hidden"></div>

  <main class="main-content w-full px-[var(--margin-x)] pb-8">  
    
  </main>
</div>
