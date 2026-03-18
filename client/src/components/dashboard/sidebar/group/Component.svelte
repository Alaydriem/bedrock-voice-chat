<script lang="ts">
    import { untrack } from "svelte";
    import type { Channel } from "../../../../js/bindings/Channel";
    import type { ChannelPlayer } from "../../../../js/bindings/ChannelPlayer";
    import type { GamerpicResponse } from "../../../../js/bindings/GamerpicResponse";
    import { debug } from '@tauri-apps/plugin-log';
    import { invoke } from '@tauri-apps/api/core';
    import GameNameUtils from "../../../../js/app/utils/GameNameUtils";
    import ImageCache from "../../../../js/app/components/imageCache";
    import ImageCacheOptions from "../../../../js/app/components/imageCacheOptions";

    interface Props {
        channel: Channel;
        currentUser: string;
        userCurrentChannelId: string | null;
        onJoin: (channelId: string) => void;
        onLeave: (channelId: string) => void;
        onDelete: (channelId: string) => void;
        onRename?: (channelId: string, newName: string) => void;
    }
    let { channel, currentUser, userCurrentChannelId, onJoin, onLeave, onDelete, onRename }: Props = $props();

    let isCurrentUserChannel = $derived(userCurrentChannelId === channel.id);
    let isOwner = $derived(currentUser && GameNameUtils.namesMatch(channel.creator, currentUser));
    let isUserInChannel = $derived(currentUser && channel.players.some(p => GameNameUtils.namesMatch(p.name, currentUser)));
    let shouldShowMenu = $derived(isUserInChannel);

    let sortedPlayers = $derived((() => {
        const owner = channel.players.find(p => GameNameUtils.namesMatch(p.name, channel.creator));
        const otherMembers = channel.players.filter(p => !GameNameUtils.namesMatch(p.name, channel.creator));

        if (owner) {
            return [owner, ...otherMembers];
        } else {
            return [...otherMembers];
        }
    })());

    let displayPlayers = $derived(sortedPlayers.slice(0, 6));
    let hasMorePlayers = $derived(sortedPlayers.length > 6);
    let additionalCount = $derived(sortedPlayers.length - 6);

    const imageCache = new ImageCache();
    let resolvedGamepics: Record<string, string> = $state({});
    let pendingFetches: Set<string> = new Set();

    function playerKey(player: ChannelPlayer): string {
        const game = player.game || GameNameUtils.extractGame(player.name);
        const gamertag = GameNameUtils.stripPrefix(player.name);
        return `${game}:${gamertag}`;
    }

    $effect(() => {
        const players = channel.players;

        untrack(() => {
            for (const player of players) {
                const key = playerKey(player);
                if (!resolvedGamepics[key] && !pendingFetches.has(key)) {
                    resolveGamepic(player, key);
                }
            }
        });
    });

    async function resolveGamepic(player: ChannelPlayer, key: string): Promise<void> {
        pendingFetches.add(key);
        const game = player.game || GameNameUtils.extractGame(player.name);
        const gamertag = GameNameUtils.stripPrefix(player.name);

        try {
            if (player.gamerpic) {
                if (player.gamerpic.startsWith('data:')) {
                    resolvedGamepics = { ...resolvedGamepics, [key]: player.gamerpic };
                    return;
                }
                try {
                    const dataUrl = await imageCache.getImage(new ImageCacheOptions(player.gamerpic, 2592000));
                    resolvedGamepics = { ...resolvedGamepics, [key]: dataUrl };
                    return;
                } catch {
                    // Fall through to API
                }
            }

            const response = await invoke<GamerpicResponse>('api_get_player_gamerpic', { game, gamertag });
            if (response.gamerpic) {
                const dataUrl = await imageCache.getImage(new ImageCacheOptions(response.gamerpic, 2592000));
                resolvedGamepics = { ...resolvedGamepics, [key]: dataUrl };
            }
        } catch (err) {
            debug(`Failed to resolve gamerpic for ${key}: ${err}`);
        } finally {
            pendingFetches.delete(key);
        }
    }

    const getAvatarClasses = (player: ChannelPlayer) => {
        const isPlayerOwner = GameNameUtils.namesMatch(player.name, channel.creator);
        const isPlayerInChannel = channel.players.some(p => GameNameUtils.namesMatch(p.name, player.name));

        if (isPlayerOwner) {
            if (isPlayerInChannel) {
                return "bg-gradient-to-br from-purple-500 to-purple-700";
            } else {
                return "bg-gradient-to-br from-red-500 to-red-700";
            }
        } else {
            return "bg-gradient-to-br from-blue-500 to-blue-700";
        }
    };

    let dropdownOpen = $state(false);
    let isEditing = $state(false);
    let editName = $state('');

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

    const handleCopyGroupId = async () => {
        await navigator.clipboard.writeText(channel.id);
        dropdownOpen = false;
    };

    const handleNameClick = (event: Event) => {
        if (isOwner && onRename) {
            event.stopPropagation();
            isEditing = true;
            editName = channel.name;
        }
    };

    const handleRenameSubmit = () => {
        const trimmed = editName.trim();
        if (trimmed && trimmed !== channel.name && onRename) {
            onRename(channel.id, trimmed);
        }
        isEditing = false;
    };

    const handleRenameKeydown = (event: KeyboardEvent) => {
        if (event.key === 'Enter') {
            handleRenameSubmit();
        } else if (event.key === 'Escape') {
            isEditing = false;
        }
    };

    const toggleDropdown = (event: Event) => {
        event.stopPropagation();
        dropdownOpen = !dropdownOpen;
    };

    const handleOutsideClick = (event: Event) => {
        if (!event.target || !(event.target as Element).closest('.group-dropdown')) {
            dropdownOpen = false;
        }
    };
</script>

<svelte:window onclick={handleOutsideClick} />

<div
    class="group relative bg-white dark:bg-navy-700 rounded-lg p-3 mb-2 shadow-soft hover:shadow-md transition-all duration-200 border border-slate-200 dark:border-navy-600 cursor-pointer {isUserInChannel ? 'ring-2 ring-indigo-500 ring-opacity-50 bg-indigo-50 dark:bg-indigo-900/20' : ''}"
    onclick={handleGroupClick}
    role="button"
    tabindex="0"
    onkeydown={(e) => e.key === 'Enter' && handleGroupClick()}
>
    <div class="flex min-w-0 flex-1 items-center justify-between gap-3">
        <div class="min-w-0 flex-1">
            {#if isEditing}
                <!-- svelte-ignore a11y_autofocus -->
                <input
                    type="text"
                    bind:value={editName}
                    onblur={handleRenameSubmit}
                    onkeydown={handleRenameKeydown}
                    onclick={(e) => e.stopPropagation()}
                    autofocus
                    class="w-full truncate text-slate-800 dark:text-navy-100 font-medium text-sm mb-2 bg-transparent border-b border-indigo-500 outline-none px-0 py-0"
                />
            {:else}
                <p
                    class="truncate text-slate-800 dark:text-navy-100 font-medium text-sm mb-2 {isOwner && onRename ? 'cursor-text hover:border-b hover:border-slate-300 dark:hover:border-navy-400' : ''}"
                    onclick={handleNameClick}
                    role={isOwner && onRename ? "button" : undefined}
                    tabindex={isOwner && onRename ? 0 : undefined}
                    onkeydown={(e) => { if (isOwner && onRename && e.key === 'Enter') handleNameClick(e); }}
                >
                   {channel.name}
                </p>
            {/if}
            <div class="flex flex-wrap -space-x-2">
                {#if sortedPlayers.length > 0}
                    {#each displayPlayers as player}
                        <div class="avatar size-7 hover:z-10 transition-transform hover:scale-110" data-player={playerKey(player)}>
                            {#if resolvedGamepics[playerKey(player)]}
                                <img
                                    src={resolvedGamepics[playerKey(player)]}
                                    alt={GameNameUtils.stripPrefix(player.name)}
                                    class="rounded-full size-7 ring-2 ring-white dark:ring-navy-700 shadow-sm object-cover"
                                />
                            {:else}
                                <div
                                    class="is-initial rounded-full {getAvatarClasses(player)} text-xs font-semibold uppercase text-white ring-2 ring-white dark:ring-navy-700 shadow-sm"
                                >
                                    {GameNameUtils.stripPrefix(player.name).slice(0, 2).toLowerCase()}
                                </div>
                            {/if}
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
                    onclick={toggleDropdown}
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
                        <button
                            class="block w-full px-4 py-2 text-left text-sm text-slate-700 hover:bg-slate-50 dark:text-navy-100 dark:hover:bg-navy-600 transition-colors duration-150"
                            onclick={handleCopyGroupId}
                        >
                            Copy Group ID
                        </button>
                        <button
                            class="block w-full px-4 py-2 text-left text-sm text-slate-700 hover:bg-slate-50 dark:text-navy-100 dark:hover:bg-navy-600 transition-colors duration-150"
                            onclick={handleLeaveGroup}
                        >
                            Leave Group
                        </button>
                        {#if isOwner}
                            <button
                                class="block w-full px-4 py-2 text-left text-sm text-red-600 hover:bg-red-50 dark:text-red-400 dark:hover:bg-red-900/20 transition-colors duration-150 font-medium"
                                onclick={handleDeleteGroup}
                            >
                                Close Group
                            </button>
                        {/if}
                    </div>
                {/if}
            </div>
        {/if}
    </div>
</div>
