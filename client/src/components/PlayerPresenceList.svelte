<script lang="ts">
    import { activePlayers, updatePlayerGain, updatePlayerMute } from '../stores/players';
    import PlayerPresence from './events/PlayerPresence.svelte';
    
    function handleGainChange(playerName: string, gain: number) {
        updatePlayerGain(playerName, gain);
    }
    
    function handleMuteToggle(playerName: string, muted: boolean) {
        updatePlayerMute(playerName, muted);
    }
</script>

<div class="player-presence-section flex-1 p-4">
    <h2 class="text-lg font-semibold mb-2 text-white">Connected Players</h2>
    
    <div class="space-y-2 max-h-64 overflow-y-auto">
        {#each $activePlayers as player (player.name)}
            <PlayerPresence 
                player={player.name}
                initialGain={player.settings.gain}
                initialMuted={player.settings.muted}
                onGainChange={(gain: number) => handleGainChange(player.name, gain)}
                onMuteToggle={(muted: boolean) => handleMuteToggle(player.name, muted)}
            />
        {:else}
            <div class="text-gray-400 text-center py-4">
                No other players connected
            </div>
        {/each}
    </div>
    
    {#if $activePlayers.length > 0}
        <div class="text-xs text-gray-500 mt-2">
            {$activePlayers.length} player{$activePlayers.length === 1 ? '' : 's'} connected
        </div>
    {/if}
</div>
