<script lang="ts">
    import { onMount } from "svelte";
    import RecordingSettings from "../../../js/app/settings/recordings";

    let showDeviceContainers = false;

    onMount(async () => {
        const settings = new RecordingSettings();
        showDeviceContainers = await settings.initialize();
    });
</script>

<div id="recordings-settings-page" class="grid grid-cols-1 gap-4 sm:gap-5 lg:gap-6 pt-4 md:pt-0">
    <div class="card">
        <div class="flex items-center justify-between py-4 px-4 sm:px-5">
            <div class="flex items-center space-x-2">
                <svg xmlns="http://www.w3.org/2000/svg" class="size-5 text-slate-400 dark:text-navy-300" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <circle cx="12" cy="12" r="10" stroke-width="2"/>
                    <circle cx="12" cy="12" r="3" stroke-width="2"/>
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6l4 2"/>
                </svg>
                <h2 class="text-base font-medium tracking-wide text-slate-700 line-clamp-1 dark:text-navy-100">
                    Recording Sessions
                </h2>
            </div>
        </div>
        <div class="px-4 pb-4 sm:px-5 sm:pb-5">
            <div id="recordings-table-container">
                <!-- Table will be rendered here -->
            </div>
        </div>
    </div>
</div>
