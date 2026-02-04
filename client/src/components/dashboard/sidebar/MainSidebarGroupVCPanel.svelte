<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { info, error as logError } from '@tauri-apps/plugin-log';
    import { onMount, onDestroy } from "svelte";
    import { uniqueNamesGenerator, adjectives, colors, animals } from 'unique-names-generator';
    import { Store } from '@tauri-apps/plugin-store';
    import Keyring from '../../../js/app/keyring';
    import GroupComponent from "./group/Component.svelte";
    import RecordButton from "./RecordButton.svelte";
    import RefreshButton from "./RefreshButton.svelte";
    import MuteButton from "./MuteButton.svelte";
    import DeafenButton from "./DeafenButton.svelte";
    import NetworkHealthIndicator from "./NetworkHealthIndicator.svelte";
    import type { Channel } from "../../../js/bindings/Channel";
    import type { PlayerManager } from "../../../js/app/managers/PlayerManager";
    import type ChannelManager from "../../../js/app/managers/ChannelManager";

    export let playerManager: PlayerManager;
    export let channelManager: ChannelManager;
    export let store: Store;
    export let serverUrl: string;
    export let onClose: (() => void) | undefined = undefined;

    let currentUser = "";
    let isListeningActive = false;
    let channels: Channel[] = [];
    let error: string | null = null;
    let isListening = false;
    let isLoading = false;

    $: channelsStore = channelManager?.channels;
    $: errorStore = channelManager?.error;
    $: isListeningStore = channelManager?.isListening;
    $: isLoadingStore = channelManager?.isLoading;
    $: currentUserStore = playerManager?.currentUser;

    $: channels = channelsStore ? $channelsStore : [];
    $: error = errorStore ? $errorStore : null;
    $: isListening = isListeningStore ? $isListeningStore : false;
    $: isLoading = isLoadingStore ? $isLoadingStore : false;
    $: currentUser = currentUserStore ? $currentUserStore : "";
    $: currentUserChannel = channels.find((channel: Channel) => channel.players && channel.players.includes(currentUser));
    $: userCurrentChannelId = currentUserChannel?.id || null;

    const initializeApiIfNeeded = async () => {
        try {
            if (!serverUrl) {
                logError("No current server configured");
                return false;
            }

            const keyring = await Keyring.new("servers");
            await keyring.setServer(serverUrl);

            const certificate = await keyring.get("certificate");
            const certificateKey = await keyring.get("certificate_key");
            const certificateCa = await keyring.get("certificate_ca");

            if (!certificate || !certificateKey || !certificateCa) {
                logError("No certificates found in keyring");
                return false;
            }

            const cert = typeof certificateCa === 'string' ? certificateCa : new TextDecoder().decode(certificateCa);
            const certKeyStr = typeof certificateKey === 'string' ? certificateKey : new TextDecoder().decode(certificateKey);
            const certStr = typeof certificate === 'string' ? certificate : new TextDecoder().decode(certificate);
            const pem = certStr + certKeyStr;

            await invoke('api_initialize_client', {
                endpoint: serverUrl,
                cert,
                pem
            });

            return true;
        } catch (error) {
            logError(`Failed to initialize API: ${error}`);
            return false;
        }
    };

    onMount(async () => {
        const apiInitialized = await initializeApiIfNeeded();

        if (apiInitialized) {
            await channelManager.startListening();
            isListeningActive = true;
            channelManager.fetchChannels();
        } else {
            logError('Failed to initialize API, channels will not be loaded');
        }
    });

    onDestroy(() => {
        if (isListeningActive) {
            channelManager.stopListening();
            isListeningActive = false;
        }
    });

    const handleNewGroup = async () => {
        try {
            const randomName = uniqueNamesGenerator({
                dictionaries: [adjectives, colors, animals],
                separator: ' ',
                style: 'capital'
            });

            if (userCurrentChannelId) {
                const currentChannel = channels.find((c: Channel) => c.id === userCurrentChannelId);
                if (currentChannel) {
                    await channelManager.leaveChannel(userCurrentChannelId, currentUser);
                }
            }

            const newChannelId = await channelManager.createChannel(randomName);
            if (newChannelId) {
                await channelManager.joinChannel(newChannelId, currentUser);
            }
        } catch (error) {
            logError(`Failed to create new group: ${error}`);
        }
    };

    const handleJoinGroup = async (channelId: string) => {
        if (!currentUser) {
            logError('No current user available');
            return;
        }
        await channelManager.joinChannel(channelId, currentUser);
    };

    const handleLeaveGroup = async (channelId: string) => {
        if (!currentUser) {
            logError('No current user available');
            return;
        }
        await channelManager.leaveChannel(channelId, currentUser);
    };

    const handleDeleteGroup = async (channelId: string) => {
        await channelManager.deleteChannel(channelId);
    };

    const clearError = () => {
        channelManager.clearError();
    };
</script>
<div class="sidebar-panel">
    <div
        class="flex h-full grow flex-col bg-white pl-[var(--main-sidebar-width)] dark:bg-navy-750"
    >
        <!-- Sidebar Panel Header -->
        <div class="flex h-18 w-full items-center justify-between pl-4 pr-1">
            <div class="flex items-center">
                <p
                    class="text-lg font-medium tracking-wider text-slate-800 line-clamp-1 dark:text-navy-100"
                >
                    Bedrock Voice Chat
                </p>
            </div>
            {#if onClose}
                <button
                    on:click={onClose}
                    aria-label="Close sidebar"
                    style="touch-action: manipulation;"
                    class="btn size-8 rounded-full p-0 hover:bg-slate-300/20 focus:bg-slate-300/20 active:bg-slate-300/25 dark:hover:bg-navy-300/20 dark:focus:bg-navy-300/20 dark:active:bg-navy-300/25 md:hidden"
                >
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="size-5">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
                    </svg>
                </button>
            {/if}
        </div>

        <!-- Sidebar Panel Body -->
        <div class="flex h-[calc(100%-4.5rem)] grow flex-col">
            <div class="mt-2 px-4">
                <button
                    class="btn w-full space-x-2 rounded-full border border-slate-200 py-2 font-medium text-slate-800 hover:bg-slate-150 focus:bg-slate-150 active:bg-slate-150/80 dark:border-navy-500 dark:text-navy-50 dark:hover:bg-navy-500 dark:focus:bg-navy-500 dark:active:bg-navy-500/90"
                    on:click={handleNewGroup}
                >
                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        fill="none"
                        viewBox="0 0 24 24"
                        stroke-width="1.5"
                        stroke="currentColor"
                        class="size-4.5"
                    >
                        <path
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            d="M12 4.5v15m7.5-7.5h-15"
                        />
                    </svg>

                    <span> New Group </span>
                </button>
            </div>

            <div class="my-4 mx-4 h-px bg-slate-200 dark:bg-navy-500"></div>
            <div class="flex flex-col grow overflow-hidden">
                <div class="flex min-w-0 items-center justify-between px-4">
                    <span class="truncate text-tiny-plus font-medium uppercase"
                        >Active Voice Channels</span
                    >
                    <!--
              <button
                class="btn -mr-1.5 size-6 rounded-full p-0 hover:bg-slate-300/20 focus:bg-slate-300/20 active:bg-slate-300/25 dark:hover:bg-navy-300/20 dark:focus:bg-navy-300/20 dark:active:bg-navy-300/25"
              >
                <svg
                  xmlns="http://www.w3.org/2000/svg"
                  class="size-3.5"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                  stroke-width="2"
                >
                  <path
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
                  ></path>
                </svg>
              </button>
            -->
                </div>
                <div
                    class="overflow-y-auto is-scrollbar-hidden min-w-0 px-2 pt-1"
                    style="min-height: 100%"
                >
                    <!-- Error Display -->
                    {#if error}
                        <div class="mb-2 rounded-lg bg-red-50 p-2 text-xs text-red-600 dark:bg-red-900/20 dark:text-red-400">
                            {error}
                            <button
                                class="ml-2 text-red-700 hover:text-red-800 dark:text-red-300 dark:hover:text-red-200"
                                on:click={clearError}
                            >
                                ×
                            </button>
                        </div>
                    {/if}

                    <!-- Channel List -->
                    {#if channels.length > 0}
                        {#each channels as channel (channel.id)}
                            <GroupComponent
                                {channel}
                                {currentUser}
                                {userCurrentChannelId}
                                onJoin={handleJoinGroup}
                                onLeave={handleLeaveGroup}
                                onDelete={handleDeleteGroup}
                            />
                        {/each}
                    {:else if isLoading}
                        <div class="flex items-center justify-center p-4 text-xs text-slate-400 dark:text-navy-400">
                            <div class="mr-2 size-3 animate-spin rounded-full border-2 border-slate-300 border-t-slate-600 dark:border-navy-500 dark:border-t-navy-300"></div>
                            Loading channels...
                        </div>
                    {:else}
                        <div class="p-4 text-center text-xs text-slate-400 dark:text-navy-400">
                            No active voice channels
                        </div>
                    {/if}
                </div>
            </div>
            <div
                class="flex h-10 shrink-0 justify-between border-t border-slate-150 px-1.5 py-1 dark:border-navy-600"
            >
                <RefreshButton disabled={false} />
                <div class="flex">
                    <RecordButton disabled={!currentUser || !isListening} />
                    <DeafenButton disabled={false} />
                    <MuteButton disabled={false} />
                    <NetworkHealthIndicator />
                </div>
            </div>
        </div>
    </div>
</div>
