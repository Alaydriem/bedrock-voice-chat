<script lang="ts">
    import { getContext } from "svelte";
    import { page } from "$app/state";
    import SettingsScreen from "../../../../components/settings/SettingsScreen.svelte";
    import { SettingsRoute } from "../../../../js/app/settings/SettingsRoute";
    import type { SettingsNavigation } from "../../../../js/app/settings/SettingsNavigation";
    import { SETTINGS_NAV_KEY } from "../../../../js/app/shell/SettingsNavigationContext";
    import { UPDATE_STATUS_KEY } from "../../../../js/app/shell/UpdateStatusContext";
    import type { UpdateStatus } from "../../../../js/app/settings/UpdateStatus";

    const pane = $derived(SettingsRoute.paneOf(page.url.pathname));

    // Owned and polled by the dashboard layout, so the badge here reflects a check that has
    // already run rather than one this screen starts on being opened.
    const updates = getContext<UpdateStatus>(UPDATE_STATUS_KEY);

    const nav = getContext<SettingsNavigation>(SETTINGS_NAV_KEY);
</script>

<SettingsScreen
    {pane}
    level="detail"
    {updates}
    onnavigate={(next) => void nav.select(next)}
    onback={() => void nav.up(page.url.pathname)}
    onclose={() => void nav.exit(page.url.pathname)}
/>
