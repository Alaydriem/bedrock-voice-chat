<script lang="ts">
    import PlayerCard from './PlayerCard.svelte';
    import { info } from '@tauri-apps/plugin-log';
    import type { PlayerSource } from '../js/bindings/PlayerSource';
    import type { PlayerManager } from '../js/app/managers/PlayerManager';
    import type { AudioActivityManager } from '../js/app/managers/AudioActivityManager';
    
    // Define PlayerData interface locally (matches PlayerManager definition)
    interface PlayerData {
        name: string;
        sources: Set<string>;
        settings: {
            gain: number;
            muted: boolean;
        };
    }
    
    // Manager props (injected via dependency injection)
    export let playerManager: PlayerManager;
    export let audioActivityManager: AudioActivityManager;

    // Store values that will be reactively updated
    let activePlayers: PlayerData[] = [];
    let currentUser = "";

    // Get store objects from managers
    $: activePlayersStore = playerManager?.activePlayers;
    $: currentUserStore = playerManager?.currentUser;

    // Subscribe to stores using proper Svelte syntax
    $: activePlayers = activePlayersStore ? $activePlayersStore : [];
    $: currentUser = currentUserStore ? $currentUserStore : "";

    function handleGainChange(playerName: string, gain: number) {
        if (playerManager) {
            playerManager.updatePlayerGain(playerName, gain);
        }
    }
    
    function handleMuteToggle(playerName: string, muted: boolean) {
        if (playerManager) {
            playerManager.updatePlayerMute(playerName, muted);
        }
    }
    
    // Helper function to check if a player is a group member
    function isPlayerGroupMember(player: PlayerData): boolean {
        return player.sources && player.sources.has('Group');
    }
</script>

<div class="player-presence-section flex-1 p-4 flex flex-col h-full">
    
    <!-- Responsive flex layout for Discord-like card layout with auto-spacing -->
    <div class="flex flex-wrap justify-evenly gap-y-4 flex-1 min-h-0">
        {#each activePlayers as player (player.name)}
            <div class="player-card-container">
                <PlayerCard 
                    player={player.name}
                    initialGain={player.settings.gain}
                    initialMuted={player.settings.muted}
                    isGroupMember={isPlayerGroupMember(player)}
                    onGainChange={(gain) => handleGainChange(player.name, gain)}
                    onMuteToggle={(muted) => handleMuteToggle(player.name, muted)}
                    {audioActivityManager}
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
    {#if activePlayers.length > 0}
        <div class="text-xs text-gray-500 mt-4 text-center shrink-0">
            <i class="fas fa-users mr-1"></i>
            {activePlayers.length} player{activePlayers.length === 1 ? '' : 's'} connected and nearby
        </div>
    {/if}
</div>
