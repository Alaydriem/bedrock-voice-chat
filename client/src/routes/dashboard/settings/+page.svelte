<script lang="ts">
    import { getContext, onMount } from "svelte";
    import { goto } from "$app/navigation";
    import { page } from "$app/state";
    import SettingsScreen from "../../../components/settings/SettingsScreen.svelte";
    import { SettingsCatalogue } from "../../../js/app/settings/SettingsCatalogue";
    import { SettingsRoute } from "../../../js/app/settings/SettingsRoute";
    import type { SettingsNavigation } from "../../../js/app/settings/SettingsNavigation";
    import { SETTINGS_NAV_KEY } from "../../../js/app/shell/SettingsNavigationContext";
    import { UPDATE_STATUS_KEY } from "../../../js/app/shell/UpdateStatusContext";
    import type { UpdateStatus } from "../../../js/app/settings/UpdateStatus";
    import PlatformDetector from "../../../js/app/utils/PlatformDetector";

    /**
     * The phone's section list, and on desktop a path that names no pane.
     *
     * Giving the list its own path is what lets the platform back climb to it: the back
     * button reads the session history, and a screen with no entry of its own is a screen
     * back cannot stop at.
     *
     * Desktop shows the section nav beside the pane, so there is no list-only screen for
     * this path to be there. It stands in for the landing pane instead, redirected
     * client-side because with `ssr = false` a `load` redirect would be a route
     * adapter-static emits and nothing ever serves. `replaceState` keeps it out of the
     * history, or climbing out of a pane would land here and bounce forward again.
     */
    const mobile = new PlatformDetector().mobile();

    const nav = getContext<SettingsNavigation>(SETTINGS_NAV_KEY);

    // Owned and polled by the dashboard layout, so the badge here reflects a check that
    // has already run rather than one this screen starts on being opened.
    const updates = getContext<UpdateStatus>(UPDATE_STATUS_KEY);

    onMount(() => {
        if (mobile) return;
        void goto(SettingsRoute.href(SettingsCatalogue.fallback), { replaceState: true });
    });
</script>

{#if mobile}
    <SettingsScreen
        pane={SettingsCatalogue.fallback}
        level="list"
        {updates}
        onnavigate={(next) => void nav.select(next)}
        onback={() => void nav.up(page.url.pathname)}
        onclose={() => void nav.exit(page.url.pathname)}
    />
{/if}
