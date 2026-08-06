<script lang="ts">
    import { getContext } from "svelte";
    import { goto } from "$app/navigation";
    import { page } from "$app/state";
    import SettingsScreen from "../../../../components/settings/SettingsScreen.svelte";
    import { SettingsRoute } from "../../../../js/app/settings/SettingsRoute";
    import { UPDATE_STATUS_KEY } from "../../../../js/app/shell/UpdateStatusContext";
    import type { UpdateStatus } from "../../../../js/app/settings/UpdateStatus";

    const pane = $derived(SettingsRoute.paneOf(page.url.pathname));

    // Owned and polled by the dashboard layout, so the badge here reflects a check that has
    // already run rather than one this screen starts on being opened.
    const updates = getContext<UpdateStatus>(UPDATE_STATUS_KEY);
</script>

<SettingsScreen
    {pane}
    {updates}
    onnavigate={(next) => void goto(SettingsRoute.href(next))}
    onclose={() => void goto("/dashboard")}
/>
