<script lang="ts">
    import type { Channel } from "../../../../js/bindings/Channel";
    import { debug, info } from '@tauri-apps/plugin-log';
    
    export let channel: Channel;
    export let currentUser: string;
    export let userCurrentChannelId: string | null;
    export let onJoin: (channelId: string) => void;
    export let onLeave: (channelId: string) => void;
    export let onDelete: (channelId: string) => void;

    $: isCurrentUserChannel = userCurrentChannelId === channel.id;
    $: isOwner = currentUser && channel.creator === currentUser;
    $: isUserInChannel = currentUser && channel.players.includes(currentUser);
    $: shouldShowMenu = isUserInChannel;

    // Debug logging for owner detection
    $: {
        if (currentUser && channel.creator) {
            debug(`Channel: ${channel.name}, Creator: ${channel.creator}, CurrentUser: ${currentUser}, IsOwner: ${isOwner}, IsUserInChannel: ${isUserInChannel}, Players: ${JSON.stringify(channel.players)}`);
        }
    }

    // Sort players so owner appears first, then regular members
    $: sortedPlayers = (() => {
        const ownerInChannel = channel.players.includes(channel.creator);
        const otherMembers = channel.players.filter(player => player !== channel.creator);
        
        if (ownerInChannel) {
            return [channel.creator, ...otherMembers];
        } else {
            // Owner has left but should still be shown first in red
            return [channel.creator, ...otherMembers];
        }
    })();

    // Generate a shorter display version of player names for UI
    $: displayPlayers = sortedPlayers.slice(0, 6);
    $: hasMorePlayers = sortedPlayers.length > 6;
    $: additionalCount = sortedPlayers.length - 6;

    // Helper function to determine avatar styling
    const getAvatarClasses = (player: string) => {
        const isPlayerOwner = player === channel.creator;
        const isPlayerInChannel = channel.players.includes(player);
        
        if (isPlayerOwner) {
            if (isPlayerInChannel) {
                // Owner is in channel - purple
                return "bg-gradient-to-br from-purple-500 to-purple-700";
            } else {
                // Owner has left - red
                return "bg-gradient-to-br from-red-500 to-red-700";
            }
        } else {
            // Regular member - blue (original color)
            return "bg-gradient-to-br from-blue-500 to-blue-700";
        }
    };

    let dropdownOpen = false;

    const handleGroupClick = () => {
        if (!isUserInChannel) {
            onJoin(channel.id);
        }
    };

    const handleLeaveGroup = () => {
        onLeave(channel.id);
        dropdownOpen = false;
    };

    const handleDeleteGroup = () => {
        onDelete(channel.id);
        dropdownOpen = false;
    };

    const toggleDropdown = (event: Event) => {
        event.stopPropagation();
        dropdownOpen = !dropdownOpen;
    };

    // Close dropdown when clicking outside
    const handleOutsideClick = (event: Event) => {
        if (!event.target || !(event.target as Element).closest('.group-dropdown')) {
            dropdownOpen = false;
        }
    };
</script>

<svelte:window on:click={handleOutsideClick} />

<div
    class="group relative bg-white dark:bg-navy-700 rounded-lg p-3 mb-2 shadow-soft hover:shadow-md transition-all duration-200 border border-slate-200 dark:border-navy-600 cursor-pointer {isUserInChannel ? 'ring-2 ring-indigo-500 ring-opacity-50 bg-indigo-50 dark:bg-indigo-900/20' : ''}"
    on:click={handleGroupClick}
    role="button"
    tabindex="0"
    on:keydown={(e) => e.key === 'Enter' && handleGroupClick()}
>
    <div class="flex min-w-0 flex-1 items-center justify-between gap-3">
        <div class="min-w-0 flex-1">
            <p class="truncate text-slate-800 dark:text-navy-100 font-medium text-sm mb-2">
               {channel.name}
            </p>
            <div class="flex flex-wrap -space-x-2">
                {#if sortedPlayers.length > 0}
                    {#each displayPlayers as player}
                        <div class="avatar size-7 hover:z-10 transition-transform hover:scale-110">
                            <div
                                class="is-initial rounded-full {getAvatarClasses(player)} text-xs font-semibold uppercase text-white ring-2 ring-white dark:ring-navy-700 shadow-sm"
                            >
                                {player.slice(0, 2).toLowerCase()}
                            </div>
                        </div>
                    {/each}
                    {#if hasMorePlayers}
                        <div class="avatar size-7 hover:z-10 transition-transform hover:scale-110">
                            <div
                                class="is-initial rounded-full bg-gradient-to-br from-slate-400 to-slate-500 text-xs font-semibold uppercase text-white ring-2 ring-white dark:ring-navy-700 shadow-sm"
                            >
                                +{additionalCount}
                            </div>
                        </div>
                    {/if}
                {:else}
                    <span class="text-xs text-slate-400 dark:text-navy-400 italic">No members</span>
                {/if}
            </div>
        </div>
        {#if shouldShowMenu}
            <div class="relative group-dropdown">
                <button
                    class="btn p-2 hover:bg-slate-200/60 focus:bg-slate-200/60 active:bg-slate-300/25 dark:hover:bg-navy-600/60 dark:focus:bg-navy-600/60 dark:active:bg-navy-300/25 opacity-0 size-8 rounded-full group-hover:opacity-100 group-focus:opacity-100 transition-all duration-200 shadow-sm"
                    on:click={toggleDropdown}
                    aria-label="Group options menu"
                >
                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke-width="1.5"
                        stroke="currentColor"
                        aria-hidden="true"
                        data-slot="icon"
                        class="size-4 stroke-2"
                    >
                        <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            d="M12 6.75a.75.75 0 1 1 0-1.5.75.75 0 0 1 0 1.5ZM12 12.75a.75.75 0 1 1 0-1.5.75.75 0 0 1 0 1.5ZM12 18.75a.75.75 0 1 1 0-1.5.75.75 0 0 1 0 1.5Z"
                        />
                    </svg>
                </button>

                {#if dropdownOpen}
                    <div class="absolute right-0 top-full mt-1 z-50 w-48 rounded-lg bg-white py-1 shadow-lg ring-1 ring-black ring-opacity-5 dark:bg-navy-700 dark:ring-navy-800 border border-slate-200 dark:border-navy-600">
                        {#if isOwner}
                            <button
                                class="block w-full px-4 py-2 text-left text-sm text-slate-700 hover:bg-slate-50 dark:text-navy-100 dark:hover:bg-navy-600 transition-colors duration-150"
                                on:click={handleLeaveGroup}
                            >
                                Leave Group
                            </button>
                            <button
                                class="block w-full px-4 py-2 text-left text-sm text-red-600 hover:bg-red-50 dark:text-red-400 dark:hover:bg-red-900/20 transition-colors duration-150 font-medium"
                                on:click={handleDeleteGroup}
                            >
                                Close Group
                            </button>
                        {:else}
                            <button
                                class="block w-full px-4 py-2 text-left text-sm text-slate-700 hover:bg-slate-50 dark:text-navy-100 dark:hover:bg-navy-600 transition-colors duration-150"
                                on:click={handleLeaveGroup}
                            >
                                Leave Group
                            </button>
                        {/if}
                    </div>
                {/if}
            </div>
        {/if}
    </div>
</div>