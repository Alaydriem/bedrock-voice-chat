<script lang="ts">
    import { activePlayers, updatePlayerGain, updatePlayerMute } from '../stores/players';
    import PlayerCard from './PlayerCard.svelte';
    
    import { playerStore } from '../stores/players'; // adjust path as needed
    
    // Initialize audio activity management
    import '../stores/audioActivity';

    // Dev helper: add a mock player to the store
    if (import.meta.env.DEV) {
        // Expose a global function for the console
        (window as any).addMockPlayer = function(name = "TestUser", gain = 1.0, muted = false) {
            playerStore.add(name, { gain, muted });
            console.log(`Added mock player: ${name}`);
        };
        
        // Expose helper to clear all players
        (window as any).clearPlayers = function() {
            playerStore.clear();
            console.log("Cleared all players");
        };
        
        // Expose helper to test audio highlighting
        (window as any).testAudioHighlight = function(playerName = "TestUser") {
            if (typeof (window as any).simulateAudioActivity === 'function') {
                (window as any).simulateAudioActivity(playerName, 1.0);
                console.log(`Testing audio highlight for ${playerName}`);
            } else {
                console.error('simulateAudioActivity function not available');
            }
        };
    }

    function handleGainChange(playerName: string, gain: number) {
        updatePlayerGain(playerName, gain);
    }
    
    function handleMuteToggle(playerName: string, muted: boolean) {
        updatePlayerMute(playerName, muted);
    }
</script>

<div class="player-presence-section flex-1 p-4 flex flex-col h-full">
    
    <!-- Responsive flex layout for Discord-like card layout with auto-spacing -->
    <div class="flex flex-wrap justify-evenly gap-y-4 flex-1 min-h-0">
        {#each $activePlayers as player (player.name)}
            <div class="player-card-container">
                <PlayerCard 
                    player={player.name}
                    initialGain={player.settings.gain}
                    initialMuted={player.settings.muted}
                    onGainChange={(gain) => handleGainChange(player.name, gain)}
                    onMuteToggle={(muted) => handleMuteToggle(player.name, muted)}
                />
            </div>
        {:else}
            <div class="col-span-full text-gray-400 text-center py-8">
                <i class="fas fa-users text-3xl mb-2 opacity-50"></i>
                <p>No other players connected</p>
                <p class="text-xs mt-1 opacity-75">Players will appear here when they join the voice channel</p>
            </div>
        {/each}
    </div>
    
    <!-- Footer with player count -->
    {#if $activePlayers.length > 0}
        <div class="text-xs text-gray-500 mt-4 text-center shrink-0">
            <i class="fas fa-users mr-1"></i>
            {$activePlayers.length} player{$activePlayers.length === 1 ? '' : 's'} connected and nearby
        </div>
    {/if}
</div>
