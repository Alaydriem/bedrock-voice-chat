<script lang="ts">
    import { onMount } from "svelte";
    import AudioLibrarySettings from "../../../js/app/settings/audioLibrary";

    onMount(async () => {
        const settings = new AudioLibrarySettings();
        await settings.initialize();
    });
</script>

<div id="audio-library-settings-page" class="grid grid-cols-1 gap-4 sm:gap-5 lg:gap-6 pt-4 md:pt-0">
    <!-- Upload Section -->
    <div class="card">
        <div class="flex items-center justify-between py-4 px-4 sm:px-5">
            <div class="flex items-center space-x-2">
                <svg xmlns="http://www.w3.org/2000/svg" class="size-5 text-slate-400 dark:text-navy-300" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"/>
                </svg>
                <h2 class="text-base font-medium tracking-wide text-slate-700 line-clamp-1 dark:text-navy-100">
                    Upload
                </h2>
            </div>
        </div>
        <div class="px-4 pb-4 sm:px-5 sm:pb-5">
            <div id="audio-library-upload-container">
                <!-- Upload component will be rendered here -->
            </div>
        </div>
    </div>

    <!-- Audio Files Table -->
    <div class="card">
        <div class="flex items-center justify-between py-4 px-4 sm:px-5">
            <div class="flex items-center space-x-2">
                <svg xmlns="http://www.w3.org/2000/svg" class="size-5 text-slate-400 dark:text-navy-300" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19V6l12-3v13M9 19c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zm12-3c0 1.105-1.343 2-3 2s-3-.895-3-2 1.343-2 3-2 3 .895 3 2zM9 10l12-3"/>
                </svg>
                <h2 class="text-base font-medium tracking-wide text-slate-700 line-clamp-1 dark:text-navy-100">
                    Audio Files
                </h2>
            </div>
        </div>
        <div class="px-4 pb-4 sm:px-5 sm:pb-5">
            <div id="audio-library-table-container">
                <!-- Table will be rendered here -->
            </div>
        </div>
    </div>
</div>
