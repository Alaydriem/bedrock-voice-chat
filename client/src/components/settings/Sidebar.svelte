<script lang="ts">
    import { mount, onMount, onDestroy } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import type { RealmsGateStatus } from "../../js/bindings/RealmsGateStatus";
    import account from "../../components/settings/pages/account.svelte";
    import audio from "../../components/settings/pages/audio.svelte";
    import keybinds from "../../components/settings/pages/keybinds.svelte";
    import recordings from "../../components/settings/pages/recordings.svelte";
    import audioLibrary from "../../components/settings/pages/audioLibrary.svelte";
    import websocket from "../../components/settings/pages/websocket.svelte";
    import proxy_connect from "../../components/settings/pages/proxy_connect.svelte";
    import realms_connect from "../../components/settings/pages/realms_connect.svelte";
    import subscriptions from "../../components/settings/pages/subscriptions.svelte";
    import about from "../../components/settings/pages/about.svelte";
    import PlatformDetector from "../../js/app/utils/PlatformDetector.ts";
    import { BedrockManager } from "../../js/app/managers/bedrock/BedrockManager";

    interface Props {
        activePage?: string;
    }

    let { activePage = "account.svelte" }: Props = $props();

    let isMobile = $state(false);
    let currentPageTitle = $state("Account");
    let realmsConnectEnabled = $state(false);

    const platformDetector = new PlatformDetector();

    let bedrockManager: BedrockManager | null = null;
    const bedrockPageIds = new Set(["proxy_connect.svelte", "realms_connect.svelte", "subscriptions.svelte"]);

    function getBedrockManager(): BedrockManager {
        if (!bedrockManager) {
            bedrockManager = new BedrockManager();
        }
        return bedrockManager;
    }

    type SidebarItem =
        | { type: "page"; id: string; title: string; icon: string; component: any }
        | { type: "separator"; label?: string };

    const settingsItems: SidebarItem[] = [
        {
            type: "page",
            id: "account.svelte",
            title: "Account",
            icon: `<svg xmlns="http://www.w3.org/2000/svg" class="size-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"/>
            </svg>`,
            component: account
        },
        {
            type: "page",
            id: "audio.svelte",
            title: "Audio Settings",
            icon: `<svg xmlns="http://www.w3.org/2000/svg" class="size-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.536 8.464a5 5 0 010 7.072m2.828-9.9a9 9 0 010 14.142M6.343 6.343L4.93 4.93a1 1 0 00-1.414 1.414l1.414 1.414a7 7 0 000 9.9L3.515 19.07a1 1 0 101.414 1.414l1.414-1.414a5 5 0 000-7.072z"/>
            </svg>`,
            component: audio
        },
        {
            type: "page",
            id: "recordings.svelte",
            title: "Recordings",
            icon: `<svg xmlns="http://www.w3.org/2000/svg" class="size-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <circle cx="12" cy="12" r="10" stroke-width="2"/>
                <circle cx="12" cy="12" r="3" stroke-width="2"/>
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6l4 2"/>
            </svg>`,
            component: recordings
        },
        {
            type: "page",
            id: "audioLibrary.svelte",
            title: "Audio Library",
            icon: `<svg xmlns="http://www.w3.org/2000/svg" class="size-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3"/>
            </svg>`,
            component: audioLibrary
        },
        {
            type: "page",
            id: "keybinds.svelte",
            title: "Keybinds",
            icon: `<svg xmlns="http://www.w3.org/2000/svg" class="size-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z"/>
            </svg>`,
            component: keybinds
        },
        {
            type: "page",
            id: "websocket.svelte",
            title: "Websocket Server",
            icon: `<svg xmlns="http://www.w3.org/2000/svg" class="size-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"/>
            </svg>`,
            component: websocket
        },
        {
            type: "page",
            id: "about.svelte",
            title: "About",
            icon: `<svg xmlns="http://www.w3.org/2000/svg" class="size-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
            </svg>`,
            component: about
        },
        { type: "separator", label: "Minecraft Bedrock" },
        {
            type: "page",
            id: "proxy_connect.svelte",
            title: "Proxy Connect",
            icon: `<svg xmlns="http://www.w3.org/2000/svg" class="size-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9"/>
            </svg>`,
            component: proxy_connect
        },
        {
            type: "page",
            id: "realms_connect.svelte",
            title: "Realms Connect",
            icon: `<svg xmlns="http://www.w3.org/2000/svg" class="size-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"/>
            </svg>`,
            component: realms_connect
        },
        {
            type: "page",
            id: "subscriptions.svelte",
            title: "Subscription",
            icon: `<svg xmlns="http://www.w3.org/2000/svg" class="size-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 10h18M7 15h1m4 0h1m-7 4h12a3 3 0 003-3V8a3 3 0 00-3-3H6a3 3 0 00-3 3v8a3 3 0 003 3z"/>
            </svg>`,
            component: subscriptions
        },
    ];

    const mobileHiddenPages = new Set(["recordings.svelte", "audioLibrary.svelte", "websocket.svelte", "keybinds.svelte"]);

    let visibleItems = $derived(
        settingsItems.filter(item => {
            if (item.type === "separator") return true;
            if (isMobile && mobileHiddenPages.has(item.id)) return false;
            if (item.id === "realms_connect.svelte" && !realmsConnectEnabled) return false;
<<<<<<< HEAD
=======
            if (item.id === "subscriptions.svelte" && !(realmsConnectEnabled && hasOffers)) return false;
>>>>>>> 86597dc (chore: initial iap variant)
            return true;
        })
    );

    function getPageConfig(pageId: string) {
        return settingsItems.find(
            (item): item is Extract<SidebarItem, { type: "page" }> =>
                item.type === "page" && item.id === pageId
        );
    }

    function mountPage(page: string, target: Document | Element | ShadowRoot) {
        const pageConfig = getPageConfig(page);
        if (pageConfig) {
            const props: Record<string, unknown> = {};
            if (bedrockPageIds.has(page)) {
                props.bedrockManager = getBedrockManager();
            }
            mount(pageConfig.component, { target, props });
        } else {
            console.warn(`No component found for page: ${page}`);
        }
    }

    function handlePageNavigation(pageId: string) {
        const pageConfig = getPageConfig(pageId);
        if (!pageConfig) return;
<<<<<<< HEAD
        // Block Realms Connect when its feature flag is off (it also renders
        // the subscription upsell internally when the user isn't entitled).
        if (pageId === "realms_connect.svelte" && !realmsConnectEnabled) {
=======
        // Block the feature-gated pages when the feature flag is off. The
        // Subscriptions page stays reachable from the banner/modal CTAs even
        // when no store offers exist (its sidebar item is offers-gated, but
        // navigation to it is not).
        if ((pageId === "realms_connect.svelte" || pageId === "subscriptions.svelte") && !realmsConnectEnabled) {
>>>>>>> 86597dc (chore: initial iap variant)
            return;
        }

        activePage = pageId;
        currentPageTitle = pageConfig.title;

        const mainElement = document.querySelector("main.settings-main-content");
        if (mainElement) {
            mainElement.innerHTML = "";
            mountPage(pageId, mainElement);
        }

        const mobileDetector = document.querySelector(".mobile-detector");
        const isMobileView = mobileDetector && window.getComputedStyle(mobileDetector).display === "block";

        if (isMobileView) {
            const navigationElement = document.querySelector(".settings-navigation");
            const contentElement = document.querySelector(".settings-main-content");
            const mobileHeader = document.querySelector(".settings-mobile-header");

            if (navigationElement && contentElement) {
                navigationElement.classList.add("nav-slide-out");
                contentElement.classList.add("content-visible");

                if (mobileHeader) {
                    mobileHeader.classList.remove("hidden");
                    mobileHeader.classList.add("flex");
                }
            }
        }
    }

    function handleBackToNavigation() {
        const mobileDetector = document.querySelector(".mobile-detector");
        const isMobileView = mobileDetector && window.getComputedStyle(mobileDetector).display === "block";

        if (isMobileView) {
            const navigationElement = document.querySelector(".settings-navigation");
            const contentElement = document.querySelector(".settings-main-content");
            const mobileHeader = document.querySelector(".settings-mobile-header");

            if (navigationElement && contentElement) {
                navigationElement.classList.remove("nav-slide-out");
                contentElement.classList.remove("content-visible");

                if (mobileHeader) {
                    mobileHeader.classList.remove("flex");
                    mobileHeader.classList.add("hidden");
                }
            }
        }
    }

    onMount(async () => {
        const mainElement = document.querySelector("main.settings-main-content");
        if (mainElement) {
            mountPage(activePage, mainElement);
        }

        const pageConfig = getPageConfig(activePage);
        if (pageConfig) {
            currentPageTitle = pageConfig.title;
        }

        try {
            isMobile = await platformDetector.checkMobile();
        } catch (error) {
            isMobile = false;
        }

        try {
            const gate = await invoke<RealmsGateStatus>("bedrock_realms_gate");
            realmsConnectEnabled = gate.status !== "feature_disabled";
        } catch (e) {
            realmsConnectEnabled = false;
        }
<<<<<<< HEAD
=======
        try {
            const offers = await invoke<unknown[]>("iap_list_offers");
            hasOffers = Array.isArray(offers) && offers.length > 0;
        } catch (e) {
            hasOffers = false;
        }

        navHandler = (e: Event) => {
            const detail = (e as CustomEvent<string>).detail;
            if (detail) handlePageNavigation(detail);
        };
        window.addEventListener("settings-navigate", navHandler as EventListener);
>>>>>>> 86597dc (chore: initial iap variant)
    });

    onDestroy(() => {
        bedrockManager?.destroy();
        if (navHandler) {
            window.removeEventListener("settings-navigate", navHandler as EventListener);
            navHandler = null;
        }
    });
</script>

<div class="settings-mobile-header md:hidden fixed top-0 left-0 right-0 z-30 h-14 items-center justify-between bg-white px-4 border-b border-slate-150 dark:bg-navy-700 dark:border-navy-600 hidden">
    <button
        class="btn size-11 rounded-full p-0 hover:bg-slate-300/20 focus:bg-slate-300/20 active:bg-slate-300/25 dark:hover:bg-navy-300/20 dark:focus:bg-navy-300/20 dark:active:bg-navy-300/25"
        onclick={handleBackToNavigation}
        aria-label="Back to settings navigation"
    >
        <svg xmlns="http://www.w3.org/2000/svg" class="size-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"/>
        </svg>
    </button>
    <h1 class="font-medium text-slate-800 dark:text-navy-100">{currentPageTitle}</h1>
    <div class="size-11"></div>
</div>

<div class="mobile-detector"></div>

<div class="settings-navigation
    fixed inset-0 z-10 w-full h-full
    md:static md:z-auto md:w-80 md:h-full md:min-h-screen md:flex-shrink-0
    sidebar sidebar-panel">
    <div class="flex h-full grow flex-col border-r border-slate-150 bg-white dark:border-navy-700 dark:bg-navy-750">

            <div class="flex items-center justify-between px-4 pt-4 h-14 md:h-18">
                <div class="hidden md:flex w-full items-center justify-between">
                    <a href="/dashboard" class="btn size-11 rounded-full p-0 hover:bg-slate-300/20 focus:bg-slate-300/20 active:bg-slate-300/25 dark:hover:bg-navy-300/20 dark:focus:bg-navy-300/20 dark:active:bg-navy-300/25" aria-label="Back to dashboard">
                        <svg xmlns="http://www.w3.org/2000/svg" class="size-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"/>
                        </svg>
                    </a>
                    <h1 class="text-xl font-semibold text-slate-800 dark:text-navy-100">Settings</h1>
                    <div class="size-11"></div>
                </div>

                <div class="flex md:hidden w-full items-center justify-between">
                    <a href="/dashboard" class="btn size-11 rounded-full p-0 hover:bg-slate-300/20 focus:bg-slate-300/20 active:bg-slate-300/25 dark:hover:bg-navy-300/20 dark:focus:bg-navy-300/20 dark:active:bg-navy-300/25" aria-label="Back to dashboard">
                        <svg xmlns="http://www.w3.org/2000/svg" class="size-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"/>
                        </svg>
                    </a>
                    <h1 class="text-lg font-medium text-slate-800 dark:text-navy-100">Settings</h1>
                    <div class="size-11"></div>
                </div>
            </div>

            <div class="nav-wrapper mt-5 h-[calc(100%-4.5rem)] overflow-x-hidden pb-6" data-simplebar>
                <div class="my-3 mx-4 h-px bg-slate-200 dark:bg-navy-500"></div>

                <ul class="flex flex-1 flex-col px-4 font-inter">
                    {#each visibleItems as item}
                        {#if item.type === "separator"}
                            <li class="mx-0 mt-5 mb-2">
                                <div class="h-px bg-slate-200 dark:bg-navy-500"></div>
                                {#if item.label}
                                    <span class="mt-2 block text-tiny+ font-semibold uppercase tracking-wider text-slate-400 dark:text-navy-300 px-1">
                                        {item.label}
                                    </span>
                                {/if}
                            </li>
                        {:else}
                            <li class="nav-item">
                                <button
                                    class="settings-nav-button flex w-full items-center space-x-3 py-3 px-4 text-left tracking-wide outline-hidden transition-all duration-300 ease-in-out rounded-lg hover:bg-slate-100 focus:bg-slate-100 dark:hover:bg-navy-600 dark:focus:bg-navy-600 min-h-[44px] md:min-h-0 relative overflow-hidden
                                        {activePage === item.id ? 'bg-primary/10 text-primary dark:bg-accent/15 dark:text-accent-light' : 'text-slate-600 hover:text-slate-800 dark:text-navy-200 dark:hover:text-navy-50'}"
                                    onclick={() => handlePageNavigation(item.id)}
                                    aria-label="Navigate to {item.title}"
                                >
                                    <div class="flex-shrink-0 text-slate-400 transition-colors {activePage === item.id ? 'text-primary dark:text-accent-light' : ''}">
                                        {@html item.icon}
                                    </div>
                                    <span class="font-medium">{item.title}</span>

                                    <div class="ml-auto md:hidden">
                                        <svg xmlns="http://www.w3.org/2000/svg" class="size-5 text-slate-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"/>
                                        </svg>
                                    </div>
                                </button>
                            </li>
                        {/if}
                    {/each}
                </ul>
            </div>
        </div>
    </div>
