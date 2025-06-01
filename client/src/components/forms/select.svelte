<script lang="ts">
    import type { AudioDevice } from "../../js/bindings/AudioDevice";
    export let label: string = "";
    export let options: Array<AudioDevice> = [];
    export let defaultOption: string = "";
    export let id: string = "";
</script>

<label class="block">
    <h2 class="text-lg font-semibold">{label}</h2>
    <select
        id="{id}"
        class="form-select mt-1 h-12 w-full rounded-lg border border-slate-300 bg-white px-2.5 text-base hover:border-slate-400 focus:border-primary dark:border-navy-450 dark:bg-navy-700 dark:hover:border-navy-400 dark:focus:border-accent"
    >
    {#each options as device}
        <option value="{device.display_name}" selected={device.display_name === defaultOption}>
            {device.display_name}
        </option>
    {/each}
    </select>
</label>