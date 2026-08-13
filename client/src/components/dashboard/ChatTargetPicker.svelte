<script lang="ts">
    import { I18n } from "$lib/i18n";
    import Icon from "$radial/components/Icon.svelte";
    import type { ChatWorld } from "../../js/bindings/ChatWorld";

    interface Props {
        options: ChatWorld[];
        current: ChatWorld;
        onPick: (world: ChatWorld) => void;
        onClose: () => void;
    }
    let { options, current, onPick, onClose }: Props = $props();

    // A hint, not a verdict. Every world stays pickable: the addon may answer again by the
    // time somebody types, and a send that is genuinely refused says so itself.
    function seen(world: ChatWorld): string {
        if (!world.available) return I18n.t("not answering right now");
        const mins = Math.round(Date.now() / 1000 - Number(world.last_seen)) / 60;
        if (mins < 2) return I18n.t("active now");
        if (mins < 60) return I18n.tf("last seen {n} min ago", { n: Math.round(mins) });
        return I18n.tf("last seen {n} h ago", { n: Math.round(mins / 60) });
    }
</script>

<!--
  Reachable only when the player is out of game with more than one world available. Standing in
  a world settles the target, and offering a choice there can only put a message in front of
  the wrong people.
-->
<div class="rad-sheet is-open" data-rad-sheet="chat-worlds">
    <span class="rad-sheet__handle"></span>
    <button class="rad-sheet__close" onclick={onClose} aria-label={I18n.t("Close world picker")}>
        <Icon name="close" />
    </button>
    <h4 class="rad-sheet__title">{I18n.t("Post to")}</h4>

    {#each options as world, i (world.world_uuid)}
        <button
            class="rad-sheet-row"
            class:is-on={world.world_uuid === current.world_uuid}
            style="--i:{i}"
            onclick={() => onPick(world)}
        >
            <span class="rad-sheet-row__text">
                <!-- world_uuid is the key and the comparison, never the label. -->
                <span class="rad-sheet-row__name">{world.world_name}</span>
                <span class="rad-sheet-row__host">{seen(world)}</span>
            </span>
            {#if world.world_uuid === current.world_uuid}
                <span class="rad-sheet-row__tick"><Icon name="check" /></span>
            {/if}
        </button>
    {/each}
</div>
