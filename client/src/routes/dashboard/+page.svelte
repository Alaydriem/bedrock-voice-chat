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
  let mainContentElement: HTMLElement;
  
  // Dashboard instance and managers
  let dashboardInstance: Dashboard | undefined;
  let playerManager: any = undefined;
  let channelManager: any = undefined;
  let audioActivityManager: any = undefined;
  
  // Initialize utilities
  const platformDetector = new PlatformDetector();
  const swipeGestureManager = new SwipeGestureManager();

  const isSidebarOpen = () => {
    return document.querySelector("body")?.classList.contains("is-sidebar-open") || false;
  };

  const openSidebar = () => {
    const currentlyOpen = isSidebarOpen();
    info(`Dashboard: openSidebar called - currently open: ${currentlyOpen}`);
    
    if (!currentlyOpen) {
      const body = document.querySelector("body");
      const toggleButton = document.querySelector("#sidebar-toggle");
      
      body?.classList.add("is-sidebar-open");
      toggleButton?.classList.add("active");
      
      info(`Dashboard: Added classes - body has is-sidebar-open: ${body?.classList.contains("is-sidebar-open")}, button has active: ${toggleButton?.classList.contains("active")}`);
      info(`Dashboard: Sidebar opened`);
    } else {
      info(`Dashboard: Sidebar already open, skipping`);
    }
  };

  const closeSidebar = () => {
    const currentlyOpen = isSidebarOpen();
    info(`Dashboard: closeSidebar called - currently open: ${currentlyOpen}`);
    
    if (currentlyOpen) {
      const body = document.querySelector("body");
      const toggleButton = document.querySelector("#sidebar-toggle");
      
      body?.classList.remove("is-sidebar-open");
      toggleButton?.classList.remove("active");
      
      info(`Dashboard: Removed classes - body has is-sidebar-open: ${body?.classList.contains("is-sidebar-open")}, button has active: ${toggleButton?.classList.contains("active")}`);
      info(`Dashboard: Sidebar closed`);
    } else {
      info(`Dashboard: Sidebar already closed, skipping`);
    }
  };

  const openGroupChatPanel = () => {
    openSidebar();
  };

  const closeGroupChatPanel = () => {
    closeSidebar();
  };

  const toggleSidebar = () => {
    info(`Dashboard: Hamburger toggle clicked`);
    const currentlyOpen = isSidebarOpen();
    info(`Dashboard: Current sidebar state - open: ${currentlyOpen}`);
    
    if (currentlyOpen) {
      info(`Dashboard: Attempting to close sidebar`);
      closeSidebar();
    } else {
      info(`Dashboard: Attempting to open sidebar`);
      openSidebar();
    }
    
    // Verify the state changed
    setTimeout(() => {
      const newState = isSidebarOpen();
      info(`Dashboard: After toggle - sidebar state: ${newState}`);
    }, 100);
  };

  const setupMobileGestures = () => {
    if (!isMobile || !isGroupChatSidebarAvailable || !mainContentElement) {
      info(`Dashboard: Skipping gesture setup - isMobile: ${isMobile}, available: ${isGroupChatSidebarAvailable}, element: ${!!mainContentElement}`);
      return;
    }

    info(`Dashboard: Setting up swipe gesture on bound element: ${mainContentElement.tagName}#${mainContentElement.id || 'no-id'}`);
    
    try {
      swipeGesture = swipeGestureManager.create({
        target: mainContentElement,
        threshold: 50, // Lower threshold for easier testing
        velocity: 0.2, // Lower velocity for easier testing
        debug: true, // Enable debug mode
        swipeLeft: ({ distance, velocity }: { distance: number; velocity: number }) => {
          info(`Dashboard: Swipe left detected - closing sidebar`);
          closeGroupChatPanel();
        },
        swipeRight: ({ distance, velocity }: { distance: number; velocity: number }) => {
          info(`Dashboard: Swipe right detected - opening sidebar`);
          openGroupChatPanel();
        },
        tap: ({ element }: { element: Element }) => {
          info(`Dashboard: Tap detected on ${element.tagName}, sidebar open: ${isSidebarOpen()}`);
          // Tap to dismiss sidebar when it's open
          if (isSidebarOpen()) {
            info(`Dashboard: Closing sidebar due to tap`);
            closeGroupChatPanel();
          }
        }
      });
      
      info(`Dashboard: Swipe gesture successfully created`);
    } catch (error) {
      info(`Dashboard: Error creating swipe gesture: ${error}`);
    }
  };

  // Reactive statement to setup gestures when conditions are met
  $: {
    info(`Dashboard: Reactive statement triggered - isMobile: ${isMobile}, isGroupChatSidebarAvailable: ${isGroupChatSidebarAvailable}, mainContentElement: ${!!mainContentElement}`);
    
    if (isMobile && isGroupChatSidebarAvailable && mainContentElement) {
      info(`Dashboard: All conditions met - calling setupMobileGestures`);
      setupMobileGestures();
    } else {
      info(`Dashboard: Conditions not met for gesture setup`);
    }
  }

  onMount(async () => {
    info(`Dashboard: Starting onMount - isGroupChatSidebarAvailable: ${isGroupChatSidebarAvailable}`);
    
    try {
      isMobile = await platformDetector.checkMobile();
      info(`Dashboard: Mobile detection result: ${isMobile}`);
    } catch (error) {
      info(`Dashboard: Mobile detection error: ${error}`);
      isMobile = false;
    }

    // For testing purposes, let's also log the user agent and temporarily force mobile
    info(`Dashboard: User agent: ${navigator.userAgent}`);
    info(`Dashboard: Screen width: ${window.innerWidth}, height: ${window.innerHeight}`);
    
    // TEMPORARY: Force mobile for testing on desktop
    if (!isMobile && window.innerWidth <= 768) {
      info(`Dashboard: Forcing mobile mode for testing (screen width <= 768)`);
      isMobile = true;
    }

    window.App = new Dashboard();
    window.dispatchEvent(new CustomEvent("app:mounted"));

    // Initialize the Dashboard first to get managers
    await window.App.initialize();
    
    // Get managers and store from the Dashboard instance
    const managers = window.App.getManagers();
    playerManager = managers.playerManager;
    channelManager = managers.channelManager;
    audioActivityManager = managers.audioActivityManager;
    const store = await Store.load("store.json", { autoSave: false });
    const serverUrl = await store.get<string>("current_server") || "";

    const mainSidebarContainer = document.getElementById(
      "main-sidebar-container",
    );

    if (mainSidebarContainer) {
      // Mount MainSidebar first
      const mainSidebarComponent = mount(MainSidebar, {
        target: mainSidebarContainer,
      });

      // Mount MainSidebarGroupVcPanel to the same container (it will position itself using pl-[var(--main-sidebar-width)])
      const groupVcPanelComponent = mount(MainSidebarGroupVcPanel, {
        target: mainSidebarContainer,
        props: {
          playerManager,
          channelManager,
          store,
          serverUrl
        }
      });

      // Now that MainSidebar is mounted, we can render the server links
      await window.App.renderSidebar(store, serverUrl);
      
      // Set the player avatar now that the DOM element exists
      window.App.setPlayerAvatar();

      if (isGroupChatSidebarAvailable) {
        openSidebar();
        closeSidebar();
      }
    }
    
    // Note: Mobile gesture setup now handled by reactive statement
    info(`Dashboard: onMount complete - waiting for mainContentElement binding`);
    
    // Initialize PlayerPresenceManager at page level
    try {
      playerPresenceManager = new PlayerPresenceManager(store, playerManager);
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
    bind:this={mainContentElement}
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
              on:click={toggleSidebar}
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
    {#if playerManager && audioActivityManager}
      <PlayerPresenceList 
        {playerManager}
        {audioActivityManager}
      />
    {/if}
  </main>
</div>
