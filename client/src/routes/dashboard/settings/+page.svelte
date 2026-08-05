<script lang="ts">
    import { onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { SettingsCatalogue } from "../../../js/app/settings/SettingsCatalogue";
    import { SettingsRoute } from "../../../js/app/settings/SettingsRoute";

    /**
     * The bare path names no pane, so it stands in for the fallback one.
     *
     * Redirected client-side rather than in `load`: with `ssr = false` the load never
     * runs at build time, so a prerendered redirect would be a route adapter-static
     * emits and nothing ever serves. `replaceState` keeps it out of the history, or
     * back from Account would land here and bounce forward again.
     */
    onMount(() => {
        void goto(SettingsRoute.href(SettingsCatalogue.fallback), { replaceState: true });
    });
</script>
