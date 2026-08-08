<script lang="ts">
  import { I18n } from "$lib/i18n";
    import Icon from "$radial/components/Icon.svelte";
    import ServerGlyph from "$radial/components/ServerGlyph.svelte";
    import type { RailServer } from "../../js/app/dashboard/RailView";

    interface Props {
        servers: readonly RailServer[];
        onswitch: (server: string) => void;
        onadd: () => void;
        onsettings: () => void;
    }
    let { servers, onswitch, onadd, onsettings }: Props = $props();

    /**
     * The glyph inside each plate.
     *
     * A canvas needs a number, so this cannot be expressed in CSS alongside the plate it sits in.
     * Stated here as a fraction of the plate rather than as its own pixel value, so widening the
     * rail cannot leave an undersized glyph adrift in the middle of a bigger tile.
     */
    const PLATE_PX = 58;
    const glyph = Math.round(PLATE_PX * 0.76);
</script>

<!--
  The kit hides this rail below 560px of frame, where the sheet takes over. There is
  therefore no phone-only duplicate of it here: one list, two presentations, and the
  breakpoint belongs to the kit rather than to this component.
-->
<div class="rad-rail">
    <div class="rad-rail__list">
        {#each servers as server (server.server)}
            <button
                class="rad-rail-item"
                class:is-on={server.isCurrent}
                title="{server.host} — signed in as {server.player}"
                aria-current={server.isCurrent ? "true" : undefined}
                onclick={() => onswitch(server.server)}
            >
                <ServerGlyph name={server.host} size={glyph} />
            </button>
        {/each}
    </div>

    <button class="rad-rail-btn" aria-label={I18n.t("Add a server")} onclick={onadd}>
        <Icon name="plus" />
    </button>

    <span class="rad-rail__spacer"></span>

    <button class="rad-rail-btn" aria-label={I18n.t("Settings")} onclick={onsettings}>
        <Icon name="gear" />
    </button>
</div>
