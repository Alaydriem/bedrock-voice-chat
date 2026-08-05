<script lang="ts">
    import { goto } from "$app/navigation";
    import { page } from "$app/state";
    import SettingsScreen from "../../../../components/settings/SettingsScreen.svelte";
    import { SettingsRoute } from "../../../../js/app/settings/SettingsRoute";

    const pane = $derived(SettingsRoute.paneOf(page.url.pathname));
</script>

<SettingsScreen
    {pane}
    onnavigate={(next) => void goto(SettingsRoute.href(next))}
    onclose={() => void goto("/dashboard")}
/>
