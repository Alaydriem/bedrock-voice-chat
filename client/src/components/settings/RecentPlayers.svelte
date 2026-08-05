<script lang="ts">
    import { onMount } from "svelte";
    import { Store } from "@tauri-apps/plugin-store";
    import Icon from "$radial/components/Icon.svelte";
    import ServerGlyph from "$radial/components/ServerGlyph.svelte";
    import {
        RecentPlayersView,
        type RecentPlayer,
    } from "../../js/app/managers/settings/RecentPlayersView";

    let view: RecentPlayersView | null = null;
    let players = $state<readonly RecentPlayer[]>([]);
    let now = $state(Date.now());

    onMount(async () => {
        const store = await Store.load("store.json", { autoSave: false, defaults: {} });
        view = new RecentPlayersView(store);
        players = await view.load();
        now = Date.now();
    });

    async function reload(): Promise<void> {
        if (view) players = await view.load();
    }

    function ago(lastSeen: number | null): string {
        if (lastSeen === null) return "not seen since this was added";
        const minutes = Math.max(0, Math.round((now - lastSeen) / 60_000));
        if (minutes < 1) return "just now";
        if (minutes < 60) return `${minutes} min ago`;
        const hours = Math.round(minutes / 60);
        if (hours < 24) return `${hours} h ago`;
        return `${Math.round(hours / 24)} d ago`;
    }
</script>

<!--
  Read from the persisted gain store rather than a list of its own: that store already holds
  exactly the players this device has an opinion about, so it answers "who have I been around"
  as well without a second list to keep in step with it.
-->
<div class="rad-settings-section">
    <div class="rad-section-rule">
        <span class="rad-section-rule__title">Players you have been near</span>
        <span class="rad-section-rule__line"></span>
        <span class="rad-section-rule__count">{players.length}</span>
    </div>

    {#if players.length === 0}
        <p class="rad-roster__empty">
            Nobody yet. Anyone who comes within earshot on a server appears here, so their
            volume is still adjustable after they have gone.
        </p>
    {:else}
        {#each players as player (player.gamertag)}
            <div class="rad-recent-row">
                <span class="rad-server-id rad-recent-row__id">
                    <ServerGlyph name={`minecraft:${player.gamertag}`.toLowerCase()} size={34} />
                </span>

                <span class="rad-recent-row__text">
                    <span class="rad-recent-row__name">{player.gamertag}</span>
                    <span class="rad-recent-row__seen">{ago(player.lastSeen)}</span>
                </span>

                <button
                    class="rad-player__mute"
                    aria-pressed={player.muted}
                    aria-label="{player.muted ? 'Unmute' : 'Mute'} {player.gamertag}"
                    onclick={async () => {
                        await view?.setMuted(player.gamertag, !player.muted);
                        await reload();
                    }}
                >
                    <Icon name={player.muted ? "micoff" : "mic"} />
                </button>

                <input
                    class="rad-range"
                    type="range"
                    min="0"
                    max="1.5"
                    step="0.05"
                    value={player.gain}
                    disabled={player.muted}
                    aria-label="Volume for {player.gamertag}"
                    onchange={async (e) => {
                        await view?.setGain(player.gamertag, Number(e.currentTarget.value));
                        await reload();
                    }}
                />
                <span class="rad-player__percent">{Math.round(player.gain * 100)}%</span>

                <button
                    class="rad-icon-btn"
                    aria-label="Forget {player.gamertag}"
                    onclick={async () => {
                        await view?.forget(player.gamertag);
                        await reload();
                    }}
                >
                    <Icon name="trash" />
                </button>
            </div>
        {/each}
    {/if}
</div>
