<script lang="ts">
    import type { LevelSource } from "$radial/core/sources/LevelSource";
    import type { NearbyPlayer } from "../../js/app/dashboard/NearbyPlayer";
    import { RosterView } from "../../js/app/dashboard/RosterView";
    import AvatarChip from "./AvatarChip.svelte";
    import PlayerCard from "./PlayerCard.svelte";

    interface Props {
        title: string;
        players: readonly NearbyPlayer[];
        /** In a channel: full volume at any distance, so no distance is shown. */
        inGroup?: boolean;
        /** Rendered into the section rule, for a group's Leave. */
        action?: import("svelte").Snippet;
        sourceFor: (name: string) => LevelSource;
        gainFor: (name: string) => number;
        mutedFor: (name: string) => boolean;
        onmute: (name: string, muted: boolean) => void;
        ongain: (name: string, gain: number) => void;
        /** The one avatar expanded for adjustment, by CN name. */
        opened: string | null;
        onopen: (name: string | null) => void;
        /** Cards whose flyer has not landed yet, by CN name. */
        pending?: ReadonlySet<string>;
    }
    let {
        title,
        players,
        inGroup = false,
        action,
        sourceFor,
        gainFor,
        mutedFor,
        onmute,
        ongain,
        opened,
        onopen,
        pending,
    }: Props = $props();

    const split = $derived(RosterView.split(players));
</script>

{#if players.length}
    <div class="rad-roster__section">
        <div class="rad-section-rule">
            <span class="rad-section-rule__title">{title}</span>
            <span class="rad-section-rule__line"></span>
            {#if action}{@render action()}{/if}
            <span class="rad-section-rule__count">{players.length}</span>
        </div>

        <div class="rad-card-grid">
            {#each split.cards as player (player.name)}
                <PlayerCard
                    {player}
                    {inGroup}
                    source={sourceFor(player.name)}
                    gain={gainFor(player.name)}
                    muted={mutedFor(player.name)}
                    pending={pending?.has(player.name) ?? false}
                    {onmute}
                    {ongain}
                />
            {/each}
        </div>

        {#if split.chips.length}
            <div class="rad-avatar-grid">
                {#each split.chips as player (player.name)}
                    <AvatarChip
                        {player}
                        source={sourceFor(player.name)}
                        open={opened === player.name}
                        onopen={(name) => onopen(opened === name ? null : name)}
                    />
                    {#if opened === player.name}
                        <!-- Expanded where it stands. Promoting it to the top of the roster
                             put the card off-screen for anyone scrolled into a long list, so
                             the tap appeared to do nothing at all. -->
                        <div class="rad-avatar-grid__open">
                            <PlayerCard
                                {player}
                                {inGroup}
                                source={sourceFor(player.name)}
                                gain={gainFor(player.name)}
                                muted={mutedFor(player.name)}
                                ondismiss={() => onopen(null)}
                                {onmute}
                                {ongain}
                            />
                        </div>
                    {/if}
                {/each}
            </div>
        {/if}
    </div>
{/if}
