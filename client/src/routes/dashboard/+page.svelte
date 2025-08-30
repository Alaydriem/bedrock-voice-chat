<script lang="ts">
  // The sidebar and related functionality is not available in this version.
  // It can be enabled by setting the variable `isGroupChatSidebarAvailable` to true.
  let isGroupChatSidebarAvailable = true;

  import MainSidebar from "../../components/dashboard/sidebar/MainSidebar.svelte";
  import MainSidebarGroupVcPanel from "../../components/dashboard/sidebar/MainSidebarGroupVCPanel.svelte";
  import PlayerPresenceList from "../../components/PlayerPresenceList.svelte";
  import "../../css/app.css";
  import Dashboard from "../../js/app/dashboard.ts";
  import { PlayerPresenceManager } from "../../js/app/components/dashboard/presence.ts";
  import { Store } from '@tauri-apps/plugin-store';
  import PlatformDetector from "../../js/app/utils/PlatformDetector";
  import SwipeGestureManager from "../../js/app/utils/SwipeGestureManager";

  import { info, error, debug } from '@tauri-apps/plugin-log';
  import { onMount, onDestroy, mount, setContext } from "svelte";

  let playerPresenceManager: PlayerPresenceManager | undefined;
  let swipeGesture: any = null;
  let isMobile = false;
  
  // Initialize utilities
  const platformDetector = new PlatformDetector();
  const swipeGestureManager = new SwipeGestureManager();

  const isSidebarOpen = () => {
    return document.querySelector("body")?.classList.contains("is-sidebar-open") || false;
  };

  const openSidebar = () => {
    if (!isSidebarOpen()) {
      document.querySelector("body")?.classList.add("is-sidebar-open");
      document.querySelector("#sidebar-toggle")?.classList.add("active");
    }
  };

  const closeSidebar = () => {
    if (isSidebarOpen()) {
      document.querySelector("body")?.classList.remove("is-sidebar-open");
      document.querySelector("#sidebar-toggle")?.classList.remove("active");
    }
  };

  const openGroupChatPanel = () => {
    openSidebar();
  };

  const closeGroupChatPanel = () => {
    closeSidebar();
  };

  const toggleSidebar = () => {
    if (isSidebarOpen()) {
      closeSidebar();
    } else {
      openSidebar();
    }
  };

  onMount(async () => {
    isMobile = await platformDetector.checkMobile();
    
    // TEMPORARY: Force mobile mode for testing (remove this later)
    isMobile = true;
    
    await debug(`Platform detection: isMobile = ${isMobile}`);

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
        openSidebar();
        closeSidebar();
      }
    }

    if (isMobile && isGroupChatSidebarAvailable) {
      
      setTimeout(() => {
        const mainContent = document.querySelector('.main-content');
        const rootElement = document.getElementById('root');
        const targetElement = mainContent || rootElement;
        
        if (targetElement) {
          swipeGesture = swipeGestureManager.create({
            target: targetElement,
            threshold: 50, // Lower threshold for easier testing
            velocity: 0.2, // Lower velocity for easier testing
            debug: true, // Enable debug mode
            swipeLeft: ({ distance, velocity }: { distance: number; velocity: number }) => {
              closeGroupChatPanel();
            },
            swipeRight: ({ distance, velocity }: { distance: number; velocity: number }) => {
              openGroupChatPanel();
            }
          });
        }
      }, 100);
    }

    await window.App.initialize();
    
    // Initialize PlayerPresenceManager at page level
    try {
      const store = await Store.load("store.json", { autoSave: false });
      playerPresenceManager = new PlayerPresenceManager(store);
      await playerPresenceManager.initialize();
      
      // Make presence manager available to child components via context
      //setContext('presenceManager', playerPresenceManager);
    } catch (err) {
      error("Failed to initialize PlayerPresenceManager" + err);
    }
  });

  onDestroy(() => {
    // Clean up swipe gesture
    if (swipeGesture) {
      swipeGesture.destroy();
    }
    
    // Clean up PlayerPresenceManager when page is destroyed
    if (playerPresenceManager) {
      playerPresenceManager.cleanup();
    }
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
    <div id="notification-container" class="notification-container"></div>
    <!-- Player Presence List - Now using reactive Svelte component -->
    <PlayerPresenceList />
  </main>
</div>
