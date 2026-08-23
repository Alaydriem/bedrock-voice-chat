<script lang="ts">
  import { I18n } from "$lib/i18n";
    import Icon from "$radial/components/Icon.svelte";
    import ServerGlyph from "$radial/components/ServerGlyph.svelte";
    import StatusChip from "$radial/components/StatusChip.svelte";
    import { ServerGlyph as Glyph } from "$radial/core/glyph/ServerGlyph";
    import type { Plate } from "../../js/app/settings/Plate";

    interface Props {
        plates: readonly Plate[];
        /** Shown as a dashed tile at the end of the grid. Omitted where you cannot add. */
        addLabel?: string;
        onconnect: (id: string) => void;
        onstop: (id: string) => void;
        onfavourite: (id: string) => void;
        onedit?: (id: string) => void;
        onremove?: (id: string) => void;
        onadd?: () => void;
    }
    let {
        plates,
        addLabel,
        onconnect,
        onstop,
        onfavourite,
        onedit,
        onremove,
        onadd,
    }: Props = $props();

    function hue(key: string): string {
        return Glyph.of(key).hue;
    }
</script>

<div class="rad-server-grid">
    {#each plates as plate (plate.id)}
        <!-- `--compact` collapses the artwork to the identity tile below 620px. -->
        <div class="rad-server rad-server--compact">
            <span
                class="rad-server__art rad-server__art--derived"
                style="--rad-server-hue: {hue(plate.glyphKey)}"
            >
                <span class="rad-server-id rad-server__id" style="width: 52px; height: 52px">
                    <ServerGlyph name={plate.glyphKey} size={52} />
                </span>
            </span>

            <span class="rad-server__body">
                <span class="rad-server__name">{plate.name}</span>
                <span class="rad-server__host">{plate.detail}</span>
                <span class="rad-server__state">
                    {#each plate.chips as chip (chip.label)}
                        <StatusChip severity={chip.severity}>{chip.label}</StatusChip>
                    {/each}
                </span>
            </span>

            <span class="rad-server__foot">
                <span style="display: flex; align-items: center; gap: 2px">
                    <button
                        class="rad-icon-btn rad-fav"
                        aria-pressed={plate.favourite}
                        onclick={() => onfavourite(plate.id)}
                        aria-label={I18n.tf("Favourite {name}", { name: plate.name })}
                    >
                        <Icon name="star" />
                    </button>
                    {#if !plate.readonly && onedit}
                        <button
                            class="rad-icon-btn"
                            onclick={() => onedit(plate.id)}
                            aria-label={I18n.tf("Edit {name}", { name: plate.name })}
                        >
                            <Icon name="field" />
                        </button>
                    {/if}
                    {#if !plate.readonly && onremove}
                        <button
                            class="rad-icon-btn"
                            onclick={() => onremove(plate.id)}
                            aria-label={I18n.tf("Remove {name}", { name: plate.name })}
                        >
                            <Icon name="trash" />
                        </button>
                    {/if}
                </span>

                {#if plate.active}
                    <button class="rad-btn rad-btn--danger" onclick={() => onstop(plate.id)}>
                        {I18n.t("Stop")}
                    </button>
                {:else}
                    <button
                        class="rad-btn {plate.reachable ? 'rad-btn--primary' : ''}"
                        disabled={!plate.reachable}
                        onclick={() => onconnect(plate.id)}
                    >
                        {I18n.t("Connect")}
                    </button>
                {/if}
            </span>
        </div>
    {/each}

    {#if addLabel && onadd}
        <button class="rad-server-add" onclick={onadd}>
            <Icon name="plus" />
            <span class="rad-server-add__label">{addLabel}</span>
        </button>
    {/if}
</div>
