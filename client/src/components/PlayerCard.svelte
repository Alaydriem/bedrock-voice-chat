<script lang="ts">
    import { audioActivity } from '../stores/audioActivity';
    
    export let player: string;
    export let initialGain: number = 1.0;
    export let initialMuted: boolean = false;
    export let onGainChange: ((gain: number) => void) | undefined = undefined;
    export let onMuteToggle: ((muted: boolean) => void) | undefined = undefined;
    
    let isMuted = initialMuted;
    let gain = initialGain;
    let showVolumeSlider = false;
    
   const cardColors = [
        'card-bg-primary',
        'card-bg-secondary',
        'card-bg-info',
        'card-bg-success',
        'card-bg-warning',
        'card-bg-error',
        'card-bg-purple',
        'card-bg-indigo',
        'card-bg-pink',
        'card-bg-teal',
        'card-bg-mustard',
        'card-bg-burnt-orange',
        'card-bg-crimson',
        'card-bg-turquoise',
        'card-bg-chartreuse',
        'card-bg-coral',
        'card-bg-slate-blue',
        'card-bg-dusty-lavender',
        'card-bg-steel-blue',
        'card-bg-taupe',
        'card-bg-charcoal',
        'card-bg-silver',
        'card-bg-ivory',
        'card-bg-electric-blue'
    ];
        
    // Function to get consistent random color for a player (deterministic based on name)
    function getPlayerCardColor(playerName: string): string {
        let hash = 0;
        for (let i = 0; i < playerName.length; i++) {
            const char = playerName.charCodeAt(i);
            hash = ((hash << 5) - hash) + char;
            hash = hash & hash;
        }
        return cardColors[Math.abs(hash) % cardColors.length];
    }
    
    const randomCardColor = getPlayerCardColor(player);
    
    function toggleMute() {
        isMuted = !isMuted;
        if (onMuteToggle) {
            onMuteToggle(isMuted);
        }
    }
    
    function updateGain() {
        if (onGainChange) {
            onGainChange(gain);
        }
    }
    
    function toggleVolumeSlider() {
        showVolumeSlider = !showVolumeSlider;
    }
    
    // Close volume slider when clicking outside the popover or button
    function handleClickOutside(event: MouseEvent) {
        if (showVolumeSlider && event.target instanceof Element && 
            !event.target.closest('.volume-popover') && 
            !event.target.closest('.volume-button')) {
            showVolumeSlider = false;
        }
    }
    
    // Close volume slider when pressing ESC key
    function handleKeydown(event: KeyboardEvent) {
        if (showVolumeSlider && event.key === 'Escape') {
            showVolumeSlider = false;
        }
    }
    
    // Reactive statement to update volume icon based on level
    $: volumeIcon = gain === 0 ? 'fa-volume-off' : 
                   gain < 0.5 ? 'fa-volume-down' : 
                   'fa-volume-up';
    
    // Reactive: update internal state when props change
    $: isMuted = initialMuted;
    $: gain = initialGain;
    
    // Reactive: check if this player is currently speaking
    $: isCurrentlySpeaking = $audioActivity.activeSpeakers[player]?.isHighlighted || false;
    
    // Debug logging (remove this later)
    $: if (import.meta.env.DEV) {
        console.log(`Player ${player} speaking state:`, isCurrentlySpeaking, $audioActivity.activeSpeakers[player]);
    }
</script>

<svelte:window on:click={handleClickOutside} on:keydown={handleKeydown} />

<!-- Gradient border wrapper with consistent sizing -->
<div class="player-card-wrapper {isCurrentlySpeaking ? 'speaking' : ''}"
     data-player={player}>
    
    <!-- Inner card content -->
    <div class="card player-card items-center text-center pb-5">
        
        <!-- Random background color overlay -->
        <div class="player-card-overlay {randomCardColor}"></div>
        
        <!-- Avatar with status indicator -->
        <div class="avatar w-20 h-20 mask is-octagon relative mx-auto mt-6">
            <div class="is-initial bg-gray-600 text-white flex items-center justify-center text-lg font-semibold">
                {player.slice(0, 3).toUpperCase()}
            </div>
        </div>
        <!-- Status indicator - positioned absolutely relative to the avatar container -->
        <div class="absolute top-5 right-1/2 transform translate-x-8 w-4 h-4 rounded-full border-2 border-white 
                    {isMuted ? 'bg-red-500' : 'bg-green-500'} 
                    dark:border-navy-700 z-20">
        </div>
        
        <!-- Player name -->
        <h3 class="mt-4 text-sm font-medium text-navy-100 dark:text-navy-100 px-2 truncate">{player}</h3>

    <!-- Floating Volume Popover (Lineone style) -->
    {#if showVolumeSlider}
        <div class="volume-popover">
            <div class="flex items-center space-x-3">
                <input 
                    type="range" 
                    min="0" 
                    max="1.5" 
                    step="0.05" 
                    bind:value={gain}
                    on:input={updateGain}
                    disabled={isMuted}
                    class="flex-1 h-2 rounded-lg appearance-none cursor-pointer 
                            bg-slate-700 border border-slate-600
                            {isMuted ? 'opacity-50' : ''}"
                />
                <span class="text-sm w-12 font-medium transition-colors {!isMuted ? 'text-slate-200' : 'text-slate-500'}">
                    {Math.round(gain * 100)}%
                </span>
            </div>
        </div>
    {/if}

        <div class="mt-4 px-4 relative">
            <div class="control-pill">
                <!-- Mute button (left side) -->
                <button class="control-pill-button {isMuted ? 'text-error' : 'text-slate-300'}"
                        on:click={toggleMute}
                        title={isMuted ? `Unmute ${player} (restore ${Math.round(gain * 100)}%)` : `Mute ${player}`}
                        aria-label={isMuted ? `Unmute ${player}` : `Mute ${player}`}>
                    <i class="fas fa-microphone{isMuted ? '-slash' : ''} text-xs"></i>
                </button>
                
                <!-- Divider -->
                <div class="control-pill-divider"></div>
                
                <!-- Volume button (right side) -->
                <button class="control-pill-button volume-button text-slate-300 {showVolumeSlider ? 'bg-slate-700' : ''}"
                        class:text-slate-500={isMuted}
                        on:click={toggleVolumeSlider}
                        title="{Math.round(gain * 100)}%"
                        aria-label="Toggle volume slider for {player}">
                    <i class="fas {volumeIcon} text-xs"></i>
                </button>
            </div>
        </div>
    </div>
</div>


