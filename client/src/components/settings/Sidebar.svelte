<script lang="ts">
    import { mount, onMount } from "svelte";
    import audio from "../../components/settings/pages/audio.svelte";
    import recordings from "../../components/settings/pages/recordings.svelte";

    export let activePage: string = "audio.svelte";

    // Page state management
    let currentPageTitle = "Audio Settings";

    // Available settings pages
    const settingsPages = [
        {
            id: "audio.svelte",
            title: "Audio Settings",
            icon: `<svg xmlns="http://www.w3.org/2000/svg" class="size-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15.536 8.464a5 5 0 010 7.072m2.828-9.9a9 9 0 010 14.142M6.343 6.343L4.93 4.93a1 1 0 00-1.414 1.414l1.414 1.414a7 7 0 000 9.9L3.515 19.07a1 1 0 101.414 1.414l1.414-1.414a5 5 0 000-7.072z"/>
            </svg>`,
            component: audio
        },
        {
            id: "recordings.svelte",
            title: "Recordings",
            icon: `<svg xmlns="http://www.w3.org/2000/svg" class="size-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <circle cx="12" cy="12" r="10" stroke-width="2"/>
                <circle cx="12" cy="12" r="3" stroke-width="2"/>
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6l4 2"/>
            </svg>`,
            component: recordings
        }
        // Add more settings pages here in the future
    ];

    function mountPage(page: string, target: Document | Element | ShadowRoot) {
        const pageConfig = settingsPages.find(p => p.id === page);
        if (pageConfig) {
            mount(pageConfig.component, { target });
        } else {
            console.warn(`No component found for page: ${page}`);
        }
    }

    function handlePageNavigation(pageId: string) {
        const pageConfig = settingsPages.find(p => p.id === pageId);
        if (!pageConfig) return;

        activePage = pageId;
        currentPageTitle = pageConfig.title;

        const mainElement = document.querySelector("main.settings-main-content");
        if (mainElement) {
            mainElement.innerHTML = ""; // Clear the main content
            mountPage(pageId, mainElement);
        }

        // CSS-driven mobile detection: check if mobile detector element is visible
        const mobileDetector = document.querySelector(".mobile-detector");
        const isMobileView = mobileDetector && window.getComputedStyle(mobileDetector).display === "block";

        if (isMobileView) {
            const navigationElement = document.querySelector(".settings-navigation");
            const contentElement = document.querySelector(".settings-main-content");
            const mobileHeader = document.querySelector(".settings-mobile-header");

            if (navigationElement && contentElement) {
                navigationElement.classList.add("nav-slide-out");
                contentElement.classList.add("content-visible");

                // Show mobile header when content is visible
                if (mobileHeader) {
                    mobileHeader.classList.remove("hidden");
                    mobileHeader.classList.add("flex");
                }
            }
        }
    }

    function handleBackToNavigation() {
        // CSS-driven mobile detection: check if mobile detector element is visible
        const mobileDetector = document.querySelector(".mobile-detector");
        const isMobileView = mobileDetector && window.getComputedStyle(mobileDetector).display === "block";

        if (isMobileView) {
            const navigationElement = document.querySelector(".settings-navigation");
            const contentElement = document.querySelector(".settings-main-content");
            const mobileHeader = document.querySelector(".settings-mobile-header");

            if (navigationElement && contentElement) {
                navigationElement.classList.remove("nav-slide-out");
                contentElement.classList.remove("content-visible");

                // Hide mobile header when returning to navigation
                if (mobileHeader) {
                    mobileHeader.classList.remove("flex");
                    mobileHeader.classList.add("hidden");
                }
            }
        }
    }

    onMount(() => {
        // Mount initial page ONCE - no duplicates
        const mainElement = document.querySelector("main.settings-main-content");
        if (mainElement) {
            mountPage(activePage, mainElement);
        }

        // Set initial page title
        const pageConfig = settingsPages.find(p => p.id === activePage);
        if (pageConfig) {
            currentPageTitle = pageConfig.title;
        }
    });
</script>
<!-- Mobile Header - Shows when content is visible on mobile -->
<div class="settings-mobile-header md:hidden fixed top-0 left-0 right-0 z-30 h-14 items-center justify-between bg-white px-4 border-b border-slate-150 dark:bg-navy-700 dark:border-navy-600 hidden">
    <button
        class="btn size-11 rounded-full p-0 hover:bg-slate-300/20 focus:bg-slate-300/20 active:bg-slate-300/25 dark:hover:bg-navy-300/20 dark:focus:bg-navy-300/20 dark:active:bg-navy-300/25"
        on:click={handleBackToNavigation}
        aria-label="Back to settings navigation"
    >
        <svg xmlns="http://www.w3.org/2000/svg" class="size-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"/>
        </svg>
    </button>
    <h1 class="font-medium text-slate-800 dark:text-navy-100">{currentPageTitle}</h1>
    <div class="size-11"></div> <!-- Spacer for center alignment -->
</div>

<!-- Mobile detection element - hidden on desktop, visible on mobile -->
<div class="mobile-detector"></div>

<!-- Navigation Panel - Responsive positioning -->
<div class="settings-navigation
    fixed inset-0 z-10 w-full h-full
    md:static md:z-auto md:w-80 md:h-full md:min-h-screen md:flex-shrink-0
    sidebar sidebar-panel">
    <div class="flex h-full grow flex-col border-r border-slate-150 bg-white dark:border-navy-700 dark:bg-navy-750">

            <!-- Desktop Header / Mobile Navigation Header -->
            <div class="flex items-center justify-between px-4 pt-4 h-14 md:h-18">
                <!-- Desktop: Settings Header with back button -->
                <div class="hidden md:flex w-full items-center justify-between">
                    <a href="/dashboard" class="btn size-11 rounded-full p-0 hover:bg-slate-300/20 focus:bg-slate-300/20 active:bg-slate-300/25 dark:hover:bg-navy-300/20 dark:focus:bg-navy-300/20 dark:active:bg-navy-300/25" aria-label="Back to dashboard">
                        <svg xmlns="http://www.w3.org/2000/svg" class="size-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"/>
                        </svg>
                    </a>
                    <h1 class="text-xl font-semibold text-slate-800 dark:text-navy-100">Settings</h1>
                    <div class="size-11"></div> <!-- Spacer for center alignment -->
                </div>

                <!-- Mobile: Settings Header with back button -->
                <div class="flex md:hidden w-full items-center justify-between">
                    <a href="/dashboard" class="btn size-11 rounded-full p-0 hover:bg-slate-300/20 focus:bg-slate-300/20 active:bg-slate-300/25 dark:hover:bg-navy-300/20 dark:focus:bg-navy-300/20 dark:active:bg-navy-300/25" aria-label="Back to dashboard">
                        <svg xmlns="http://www.w3.org/2000/svg" class="size-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"/>
                        </svg>
                    </a>
                    <h1 class="text-lg font-medium text-slate-800 dark:text-navy-100">Settings</h1>
                    <div class="size-11"></div> <!-- Spacer -->
                </div>
            </div>

            <!-- Navigation Content -->
            <div class="nav-wrapper mt-5 h-[calc(100%-4.5rem)] overflow-x-hidden pb-6" data-simplebar>
                <div class="my-3 mx-4 h-px bg-slate-200 dark:bg-navy-500"></div>

                <!-- Settings Navigation List -->
                <ul class="flex flex-1 flex-col px-4 font-inter">
                    {#each settingsPages as page}
                    <li class="nav-item">
                        <button
                            class="settings-nav-button flex w-full items-center space-x-3 py-3 px-4 text-left tracking-wide outline-hidden transition-all duration-300 ease-in-out rounded-lg hover:bg-slate-100 focus:bg-slate-100 dark:hover:bg-navy-600 dark:focus:bg-navy-600 min-h-[44px] md:min-h-0 relative overflow-hidden
                                {activePage === page.id ? 'bg-primary/10 text-primary dark:bg-accent/15 dark:text-accent-light' : 'text-slate-600 hover:text-slate-800 dark:text-navy-200 dark:hover:text-navy-50'}"
                            on:click={() => handlePageNavigation(page.id)}
                            aria-label="Navigate to {page.title}"
                        >
                            <div class="flex-shrink-0 text-slate-400 transition-colors {activePage === page.id ? 'text-primary dark:text-accent-light' : ''}">
                                {@html page.icon}
                            </div>
                            <span class="font-medium">{page.title}</span>

                            <!-- Mobile chevron -->
                            <div class="ml-auto md:hidden">
                                <svg xmlns="http://www.w3.org/2000/svg" class="size-5 text-slate-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"/>
                                </svg>
                            </div>
                        </button>
                    </li>
                    {/each}
                </ul>
            </div>
        </div>
    </div>
