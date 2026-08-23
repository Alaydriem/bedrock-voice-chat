<script lang="ts">
  import { I18n } from "$lib/i18n";
    import Icon from "$radial/components/Icon.svelte";
    import ServerGlyph from "$radial/components/ServerGlyph.svelte";
    import type { LevelSource } from "$radial/core/sources/LevelSource";
    import { onDestroy } from "svelte";
    import type { NearbyPlayer } from "../../js/app/dashboard/NearbyPlayer";

    interface Props {
        player: NearbyPlayer;
        source?: LevelSource;
        open?: boolean;
        onopen: (name: string) => void;
    }
    let { player, source, open = false, onopen }: Props = $props();

    /**
     * The level, as a custom property rather than a canvas.
     *
     * Forty avatars would be forty canvases on the animation loop painting the same thing at
     * different amplitudes. One style write each, at whatever rate the source pushes, costs
     * nothing and looks the same.
     */
    let level = $state(0);
    let off: (() => void) | null = null;

    $effect(() => {
        off?.();
        off = source?.subscribe((next) => (level = next)) ?? null;
    });

    onDestroy(() => off?.());

    const offVoice = $derived(player.presence === "game");
</script>

<button
    class="rad-avatar-chip"
    class:rad-avatar-chip--game={offVoice}
    class:is-pinned={open}
    style="color: {player.hue}; --level: {offVoice ? 0 : level}"
    aria-expanded={open}
    aria-label={I18n.tf("Adjust {gamertag}", { gamertag: player.gamertag })}
    onclick={() => onopen(player.name)}
>
    <span class="rad-avatar-chip__face">
        <!-- Lowercased: the hue beside this glyph is derived from the same key, and two
             derivations of one identity must not disagree about its colour. -->
        <span class="rad-server-id rad-avatar-chip__id">
            <ServerGlyph name={player.name.toLowerCase()} size={46} />
        </span>
        <span class="rad-avatar-chip__ring"></span>
        {#if offVoice}
            <span class="rad-avatar-chip__badge"><Icon name="unlink" /></span>
        {/if}
    </span>
    <span class="rad-avatar-chip__name">{player.gamertag}</span>
    <span class="rad-avatar-chip__meta">{Math.round(player.distance)} M</span>
</button>
