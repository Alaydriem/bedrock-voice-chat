<script lang="ts">
    import { mount, onMount } from "svelte";
    import audio from "../../components/settings/pages/audio.svelte";

    export let activePage: string = "audio.svelte";
    function mountPage(page: string, target: Document | Element | ShadowRoot) {
        switch (page) {
            case "audio.svelte":
                mount(audio, { target });
                break;
            // Add more cases for other pages if needed
            default:
                console.warn(`No component found for page: ${page}`);
        }
    }

    onMount(() => {
        const mainElement = document.querySelector("main");
        mountPage(activePage, mainElement!);
        const elements = document.querySelectorAll(".sidebar button");
        elements.forEach((element) => {
            element.addEventListener("click", (e) => {
                mainElement!.innerHTML = ""; // Clear the main content
                mountPage(activePage, mainElement!);
            });
        });
    });
</script>
<div class="sidebar sidebar-panel">
    <div
        class="flex h-full grow flex-col border-r border-slate-150 bg-white dark:border-navy-700 dark:bg-navy-750"
    >
        <div class="flex items-center justify-between px-3 pt-4">
            <!-- Application Logo -->
            <div class="flex pt-4">
                <a href="/dashboard">
                    <img
                        class="size-11 transition-transform duration-500 ease-in-out hover:rotate-[360deg]"
                        src="images/app-logo.png"
                        alt="logo"
                    />
                </a>
            </div>
        </div>

        <div
            class="nav-wrapper mt-5 h-[calc(100%-4.5rem)] overflow-x-hidden pb-6"
            data-simplebar
        >
            <div class="my-3 mx-4 h-px bg-slate-200 dark:bg-navy-500"></div>
            <ul class="flex flex-1 flex-col px-4 font-inter">
                <li
                    class="ac nav-parent [&.is-active_svg]:rotate-90 [&.is-active_.ac-trigger]:font-semibold [&.is-active_.ac-trigger]:text-slate-800 dark:[&.is-active_.ac-trigger]:text-navy-50"
                >
                    <button
                        id="audio.svelte"
                        class="ac-trigger flex w-full items-center justify-between py-2 text-xs-plus tracking-wide text-slate-600 outline-hidden transition-[color,padding-left] duration-300 ease-in-out hover:text-slate-800 dark:text-navy-200 dark:hover:text-navy-50"
                    >
                        <span>Audio Settings</span>
                        <svg
                            xmlns="http://www.w3.org/2000/svg"
                            class="size-4 text-slate-400 transition-transform ease-in-out"
                            fill="none"
                            viewBox="0 0 24 24"
                            stroke="currentColor"
                        >
                            <path
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                stroke-width="2"
                                d="M9 5l7 7-7 7"
                            ></path>
                        </svg>
                    </button>
                </li>
            </ul>
        </div>
    </div>
</div>
