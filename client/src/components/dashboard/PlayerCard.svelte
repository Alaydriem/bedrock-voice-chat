<script lang="ts">
    import Icon from "$radial/components/Icon.svelte";
    import LevelMeter from "$radial/components/LevelMeter.svelte";
    import ServerGlyph from "$radial/components/ServerGlyph.svelte";
    import type { LevelSource } from "$radial/core/sources/LevelSource";
    import type { NearbyPlayer } from "../../js/app/dashboard/NearbyPlayer";

    interface Props {
        player: NearbyPlayer;
        source?: LevelSource;
        gain: number;
        muted: boolean;
        /** Present on a card opened from an avatar, absent on one that is always a card. */
        ondismiss?: () => void;
        onmute: (name: string, muted: boolean) => void;
        ongain: (name: string, gain: number) => void;
        /** In a channel, where distance does not apply. */
        inGroup?: boolean;
        /** Held back until this card's flyer lands on it. */
        pending?: boolean;
    }
    let {
        player,
        source,
        gain,
        muted,
        ondismiss,
        onmute,
        ongain,
        inGroup = false,
        pending = false,
    }: Props = $props();

    const offVoice = $derived(player.presence === "game");
    const range = $derived(inGroup ? "IN GROUP" : `${Math.round(player.distance)} M`);
</script>

<!-- `data-card` is how the handoff finds this card's avatar to fly a mark onto. The name, not
     the gamertag: it is the key everything else on this screen is identified by. -->
<div
    class="rad-player"
    class:rad-player--game={offVoice}
    class:is-pending={pending}
    data-card={player.name}
    style="color: {player.hue}"
>
    <div class="rad-player__avatar">
        <span class="rad-server-id rad-player__id">
            <ServerGlyph name={player.name.toLowerCase()} size={54} />
        </span>
        <span class="rad-player__ring"></span>
    </div>

    <div>
        <div class="rad-player__head">
            <span class="rad-player__name">{player.gamertag}</span>
            <span class="rad-player__distance">{range}</span>
            {#if ondismiss}
                <button class="rad-player__dismiss" aria-label="Close this card" onclick={ondismiss}>
                    <Icon name="close" />
                </button>
            {/if}
        </div>

        {#if offVoice}
            <!-- No gain controls rather than disabled ones: there is no audio to turn down,
                 and a greyed slider reads as "muted", which is a different and wrong story. -->
            <div class="rad-player__note">
                <Icon name="unlink" />
                In range, not on voice &mdash; they will not hear you
            </div>
        {:else}
            <div class="rad-player__level">
                <LevelMeter source={muted ? undefined : source} color={player.hue} cell={3} />
            </div>
            <div class="rad-player__controls">
                <button
                    class="rad-player__mute"
                    aria-pressed={muted}
                    aria-label="{muted ? 'Unmute' : 'Mute'} {player.gamertag}"
                    onclick={() => onmute(player.name, !muted)}
                >
                    <Icon name={muted ? "micoff" : "mic"} />
                </button>
                <input
                    class="rad-range"
                    type="range"
                    min="0"
                    max="1.5"
                    step="0.05"
                    value={gain}
                    disabled={muted}
                    aria-label="Volume for {player.gamertag}"
                    oninput={(e) => ongain(player.name, Number(e.currentTarget.value))}
                />
                <span class="rad-player__percent">{Math.round(gain * 100)}%</span>
            </div>
        {/if}
    </div>
</div>
