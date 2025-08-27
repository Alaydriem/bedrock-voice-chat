<script lang="ts">
    export let player: string;
    export let initialGain: number = 1.0;
    export let initialMuted: boolean = false;
    export let onGainChange: ((gain: number) => void) | undefined = undefined;
    export let onMuteToggle: ((muted: boolean) => void) | undefined = undefined;
    
    let isMuted = initialMuted;
    let gain = initialGain;
    let showVolumeSlider = false;
    
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
        // If user adjusts gain while muted, auto-unmute
        if (isMuted && gain > 0) {
            toggleMute();
        }
    }
    
    function toggleVolumeSlider() {
        showVolumeSlider = !showVolumeSlider;
    }
    
    // Reactive statement to update volume icon based on level
    $: volumeIcon = gain === 0 ? 'fa-volume-off' : 
                   gain < 0.5 ? 'fa-volume-down' : 
                   'fa-volume-up';
</script>

<div class="player-card bg-navy-800 rounded-lg p-3 mb-2 border border-navy-600 transition-all duration-200"
     class:opacity-60={isMuted}
     data-player={player}>
    <div class="flex items-center justify-between">
        <!-- Player Info -->
        <div class="flex items-center space-x-2">
            <span class="inline-block h-2 w-2 rounded-full transition-colors duration-200"
                  class:bg-green-500={!isMuted}
                  class:bg-gray-500={isMuted}></span>
            <span class="text-sm font-medium transition-colors duration-200"
                  class:text-navy-100={!isMuted}
                  class:text-navy-400={isMuted}>{player}</span>
            {#if isMuted}
                <span class="text-xs text-error">(muted)</span>
            {/if}
        </div>
        
        <!-- Controls -->
        <div class="flex items-center space-x-2">
            <!-- Mute Button -->
            <button 
                class="p-1 hover:bg-navy-700 rounded transition-colors"
                class:text-error={isMuted}
                class:text-navy-300={!isMuted}
                on:click={toggleMute}
                title={isMuted ? `Unmute (restore ${Math.round(gain * 100)}%)` : 'Mute'}
                aria-label={isMuted ? `Unmute ${player}` : `Mute ${player}`}
            >
                <i class="fas fa-microphone{isMuted ? '-slash' : ''} text-xs"></i>
            </button>
            
            <!-- Volume Button -->
            <button 
                class="p-1 hover:bg-navy-700 rounded transition-colors text-navy-300"
                class:text-navy-500={isMuted}
                on:click={toggleVolumeSlider}
                title="Gain: {Math.round(gain * 100)}%"
                aria-label="Toggle volume slider for {player}"
            >
                <i class="fas {volumeIcon} text-xs"></i>
            </button>
        </div>
    </div>
    
    <!-- Volume Slider (when expanded) -->
    {#if showVolumeSlider}
        <div class="mt-3 pt-2 border-t border-navy-600 transition-all duration-200">
            <div class="flex items-center space-x-2">
                <span class="text-xs text-navy-400">Gain:</span>
                <input 
                    type="range" 
                    min="0" 
                    max="1.5" 
                    step="0.05" 
                    bind:value={gain}
                    on:input={updateGain}
                    disabled={isMuted}
                    class="flex-1 h-1 rounded-lg appearance-none cursor-pointer transition-opacity"
                    class:bg-navy-600={!isMuted}
                    class:bg-navy-700={isMuted}
                    class:opacity-50={isMuted}
                />
                <span class="text-xs w-12 transition-colors"
                      class:text-navy-400={!isMuted}
                      class:text-navy-500={isMuted}>
                    {Math.round(gain * 100)}%
                </span>
            </div>
            {#if isMuted}
                <div class="mt-1">
                    <span class="text-xs text-navy-500">Gain preserved - click unmute to restore</span>
                </div>
            {/if}
        </div>
    {/if}
</div>

<style>
    /* Custom slider styling */
    input[type="range"]::-webkit-slider-thumb {
        appearance: none;
        height: 12px;
        width: 12px;
        border-radius: 50%;
        background: #3b82f6;
        cursor: pointer;
        transition: background-color 0.2s;
    }
    
    input[type="range"]:disabled::-webkit-slider-thumb {
        background: #64748b;
        cursor: not-allowed;
    }
    
    input[type="range"]::-moz-range-thumb {
        height: 12px;
        width: 12px;
        border-radius: 50%;
        background: #3b82f6;
        cursor: pointer;
        border: none;
        transition: background-color 0.2s;
    }
    
    input[type="range"]:disabled::-moz-range-thumb {
        background: #64748b;
        cursor: not-allowed;
    }
</style>