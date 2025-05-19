<script lang="ts">
  // The sidebar and related functionality is not available in this version.
  // It can be enabled by setting the variable `isGroupChatSidebarAvailable` to true.
  const isGroupChatSidebarAvailable = true;

  import MainSidebar from "../../components/dashboard/sidebar/MainSidebar.svelte";
  import MainSidebarGroupVcPanel from "../../components/dashboard/sidebar/MainSidebarGroupVCPanel.svelte";
  import "../../css/app.css";
  import Dashboard from "../../js/app/dashboard.ts";

  import { onMount, mount } from "svelte";

  onMount(() => {
    window.App = new Dashboard();
    window.dispatchEvent(new CustomEvent("app:mounted"));

    const mainSidebarContainer = document.getElementById(
      "main-sidebar-container",
    );

    if (mainSidebarContainer) {
      mount(MainSidebar, {
        target: mainSidebarContainer,
      });

      mount(MainSidebarGroupVcPanel, {
        target: mainSidebarContainer,
      });

      if (isGroupChatSidebarAvailable) {
        document.querySelector("body")?.classList.add("is-sidebar-open");
      } else {
        document.querySelector("body")?.classList.remove("is-sidebar-open");
      }
    }

    window.App.initialize();
  });
</script>

<div id="root" class="min-h-100vh cloak flex grow bg-slate-50 dark:bg-navy-900">
  <div id="main-sidebar-container" class="sidebar print:hidden"></div>

  <main
    class="main-content chat-app h-100vh mt-0 flex flex-col w-full min-w-0 supports-[height:1dvh]:h-dvh"
  >
    <div
      class="chat-header relative z-10 flex h-[61px] w-full shrink-0 items-center justify-between border-b border-slate-150 bg-white px-[calc(var(--margin-x)-.5rem)] shadow-xs transition-[padding,width] duration-[.25s] dark:border-navy-700 dark:bg-navy-800"
    >
      {#if isGroupChatSidebarAvailable}
        <div class="flex min-w-0 items-center gap-1">
          <div class="ml-1 size-7">
            <button
              id="sidebar-toggle"
              aria-label="Toggle sidebar"
              class="menu-toggle cursor-pointer ml-0.5 flex size-7 flex-col justify-center space-y-1.5 text-primary outline-hidden focus:outline-hidden dark:text-accent-light/80 active"
            >
              <span></span>
              <span></span>
              <span></span>
            </button>
          </div>
        </div>
      {/if}
      <div class="flex space-x-3 items-center">
        <div class="flex space-x-2"></div>
      </div>
    </div>
  </main>
</div>
