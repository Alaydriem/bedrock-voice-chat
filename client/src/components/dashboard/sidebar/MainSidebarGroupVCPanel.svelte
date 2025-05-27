<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { onMount } from "svelte";

    onMount(() => {
        window.App.bindSidebarEvents();
    });
</script>
<div class="sidebar-panel">
    <div
        class="flex h-full grow flex-col bg-white pl-[var(--main-sidebar-width)] dark:bg-navy-750"
    >
        <!-- Sidebar Panel Header -->
        <div class="flex h-18 w-full items-center justify-between pl-4 pr-1">
            <div class="flex items-center">
                <p
                    class="text-lg font-medium tracking-wider text-slate-800 line-clamp-1 dark:text-navy-100"
                >
                    Bedrock Voice Chat
                </p>
            </div>
        </div>

        <!-- Sidebar Panel Body -->
        <div class="flex h-[calc(100%-4.5rem)] grow flex-col">
            <div class="mt-2 px-4">
                <button
                    class="btn w-full space-x-2 rounded-full border border-slate-200 py-2 font-medium text-slate-800 hover:bg-slate-150 focus:bg-slate-150 active:bg-slate-150/80 dark:border-navy-500 dark:text-navy-50 dark:hover:bg-navy-500 dark:focus:bg-navy-500 dark:active:bg-navy-500/90"
                >
                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke-width="1.5"
                        stroke="currentColor"
                        class="size-4.5"
                    >
                        <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            d="M12 4.5v15m7.5-7.5h-15"
                        />
                    </svg>

                    <span> New Group </span>
                </button>
            </div>

            <div class="my-4 mx-4 h-px bg-slate-200 dark:bg-navy-500"></div>
            <div class="flex flex-col grow overflow-hidden">
                <div class="flex min-w-0 items-center justify-between px-4">
                    <span class="truncate text-tiny-plus font-medium uppercase"
                        >Active Voice Channels</span
                    >
                    <!--
              <button
                class="btn -mr-1.5 size-6 rounded-full p-0 hover:bg-slate-300/20 focus:bg-slate-300/20 active:bg-slate-300/25 dark:hover:bg-navy-300/20 dark:focus:bg-navy-300/20 dark:active:bg-navy-300/25"
              >
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  class="size-3.5"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                  stroke-width="2"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
                  ></path>
                </svg>
              </button>
            -->
                </div>
                <div
                    class="overflow-y-auto is-scrollbar-hidden min-w-0 px-2 pt-1"
                >
                    <div
                        role="button"
                        class="group flex space-x-2 w-full min-w-0 items-start justify-start p-2 text-xs-plus rounded-lg tracking-wide text-slate-800 outline-hidden transition-all hover:bg-slate-100 focus:bg-slate-100 dark:text-navy-100 dark:hover:bg-navy-600 dark:focus:bg-navy-600"
                    >
                        <div
                            class="-mt-px flex min-w-0 flex-1 items-center justify-between gap-2 text-start"
                        >
                            <div class="min-w-0 flex-1 pb-2">
                                <p
                                    class="truncate text-slate-700 dark:text-navy-100 pb-2 pt-1"
                                >
                                    Example Group Voice Channel
                                </p>
                                <div class="flex flex-wrap -space-x-2">
                                    <div class="avatar size-8 hover:z-10">
                                        <div
                                            class="is-initial rounded-full bg-info text-xs-plus uppercase text-white ring-2 ring-white dark:ring-navy-700"
                                        >
                                            jd
                                        </div>
                                    </div>

                                    <div class="avatar size-8 hover:z-10">
                                        <div
                                            class="is-initial rounded-full bg-info text-xs-plus uppercase text-white ring-2 ring-white dark:ring-navy-700"
                                        >
                                            jd
                                        </div>
                                    </div>

                                    <div class="avatar size-8 hover:z-10">
                                        <div
                                            class="is-initial rounded-full bg-info text-xs-plus uppercase text-white ring-2 ring-white dark:ring-navy-700"
                                        >
                                            jd
                                        </div>
                                    </div>

                                    <div class="avatar size-8 hover:z-10">
                                        <div
                                            class="is-initial rounded-full bg-info text-xs-plus uppercase text-white ring-2 ring-white dark:ring-navy-700"
                                        >
                                            jd
                                        </div>
                                    </div>

                                    <div class="avatar size-8 hover:z-10">
                                        <div
                                            class="is-initial rounded-full bg-info text-xs-plus uppercase text-white ring-2 ring-white dark:ring-navy-700"
                                        >
                                            jd
                                        </div>
                                    </div>

                                    <div class="avatar size-8 hover:z-10">
                                        <div
                                            class="is-initial rounded-full bg-info text-xs-plus uppercase text-white ring-2 ring-white dark:ring-navy-700"
                                        >
                                            jd
                                        </div>
                                    </div>
                                </div>
                            </div>
                            <button
                                class="btn p-0 hover:bg-slate-300/20 focus:bg-slate-300/20 active:bg-slate-300/25 dark:hover:bg-navy-300/20 dark:focus:bg-navy-300/20 dark:active:bg-navy-300/25 opacity-0 size-5 rounded-full group-hover:opacity-100 group-focus:opacity-100 -mr-1"
                            >
                                <svg
                                    xmlns="http://www.w3.org/2000/svg"
                                    fill="none"
                                    viewBox="0 0 24 24"
                                    stroke-width="1.5"
                                    stroke="currentColor"
                                    aria-hidden="true"
                                    data-slot="icon"
                                    class="size-4 stroke-2"
                                >
                                    <path
                                        stroke-linecap="round"
                                        stroke-linejoin="round"
                                        d="M12 6.75a.75.75 0 1 1 0-1.5.75.75 0 0 1 0 1.5ZM12 12.75a.75.75 0 1 1 0-1.5.75.75 0 0 1 0 1.5ZM12 18.75a.75.75 0 1 1 0-1.5.75.75 0 0 1 0 1.5Z"
                                    />
                                </svg>
                            </button>
                        </div>
                    </div>
                </div>
            </div>
            <div
                class="flex h-10 shrink-0 justify-between border-t border-slate-150 px-1.5 py-1 dark:border-navy-600"
            >
                <button
                    data-tooltip="Refresh"
                    aria-label="Refresh"
                    id="reload-audio-engine"
                    x-tooltip.on.mouseenter="'Reload Audio Engine'"
                    class="pl-2 pr-2 btn size-8 rounded-full p-0 hover:bg-slate-300/20 focus:bg-slate-300/20 active:bg-slate-300/25 dark:hover:bg-navy-300/20 dark:focus:bg-navy-300/20 dark:active:bg-navy-300/25"
                >
                    <i class="fa-solid fa-arrows-rotate"></i>
                </button>
                <div class="flex">
                    <button
                        class="btn bg-slate-150 font-medium text-slate-800 hover:bg-slate-200 focus:bg-slate-200 active:bg-slate-200/80 dark:bg-navy-500 dark:text-navy-50 dark:hover:bg-navy-450 dark:focus:bg-navy-450 dark:active:bg-navy-450/90"
                    >
                        REC
                    </button>
                    <button
                        data-tooltip="Toggle Deafen"
                        aria-label="Toggle Deeafen"
                        x-tooltip.on.mouseenter="'Mute Audio Output'"
                        id="mute-audio-output"
                        class="btn size-8 rounded-full p-0 hover:bg-slate-300/20 focus:bg-slate-300/20 active:bg-slate-300/25 dark:hover:bg-navy-300/20 dark:focus:bg-navy-300/20 dark:active:bg-navy-300/25"
                    >
                        <i class="fa-solid fa-volume-high"></i>
                    </button>
                    <button
                        data-tooltip="Toggle Mute"
                        aria-label="Toggle Mute"
                        x-tooltip.on.mouseenter="'Mute Microphone'"
                        id="mute-audio-input"
                        class="btn size-8 rounded-full p-0 hover:bg-slate-300/20 focus:bg-slate-300/20 active:bg-slate-300/25 dark:hover:bg-navy-300/20 dark:focus:bg-navy-300/20 dark:active:bg-navy-300/25"
                    >
                        <i class="fa-solid fa-microphone"></i>
                    </button>
                    <button
                        data-tooltip="Help"
                        aria-label="Help"
                        class="btn size-8 rounded-full p-0 hover:bg-slate-300/20 focus:bg-slate-300/20 active:bg-slate-300/25 dark:hover:bg-navy-300/20 dark:focus:bg-navy-300/20 dark:active:bg-navy-300/25"
                    >
                        <i class="fa-regular fa-circle-question"></i>
                    </button>
                </div>
            </div>
        </div>
    </div>
</div>
