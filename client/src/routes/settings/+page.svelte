<script lang="ts">
    import "../../css/app.css";
    import { goto } from "$app/navigation";
    import { page } from "$app/state";
    import RadFrame from "../../components/shell/RadFrame.svelte";
    import SettingsScreen from "../../components/settings/SettingsScreen.svelte";
    import Notification from "../../components/events/Notification.svelte";
    import { SettingsCatalogue } from "../../js/app/settings/SettingsCatalogue";

    /**
     * Settings with no dashboard behind it.
     *
     * The error screen sends people here — "change audio devices" from a fault that has
     * already taken the dashboard down — so there is nothing to present this over and
     * nothing to dismiss back to. The pane comes from the hash rather than the path
     * because that is the shape of the links already out there.
     */
    let pane = $state(SettingsCatalogue.fallback);

    /**
     * Which screen is showing on a phone.
     *
     * Held here rather than read from the path: this route carries its pane in the hash,
     * and there is no dashboard behind it for a second history entry to sit above. Back
     * therefore leaves outright from either screen, which is what it already did.
     */
    let level = $state<"list" | "detail">("list");

    $effect(() => {
        const asked = page.url.hash.replace(/^#/, "");
        pane = SettingsCatalogue.find(asked, false)?.id ?? SettingsCatalogue.fallback;
    });
</script>

<RadFrame>
    <SettingsScreen
        {pane}
        {level}
        standalone
        onnavigate={(next) => {
            pane = next;
            level = "detail";
        }}
        onback={() => void goto("/dashboard")}
        onclose={() => void goto("/dashboard")}
    />
</RadFrame>

<Notification />
