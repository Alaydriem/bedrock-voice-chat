import { writable, derived, get, type Writable, type Readable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { info, error as logError, debug, warn } from '@tauri-apps/plugin-log';
import { Store } from '@tauri-apps/plugin-store';
import type { Channel } from '../../bindings/Channel';
import type { ChannelEvent } from '../../bindings/ChannelEvent';
import type { ChannelEvents } from '../../bindings/ChannelEvents';
import type { PlayerSource } from '../../bindings/PlayerSource';
import type { PlayerManager } from './PlayerManager';

interface ChannelStoreState {
    channels: Channel[];
    currentUserChannelId: string | null;
    isListening: boolean;
    isLoading: boolean;
    lastFetchTime: number | null;
    error: string | null;
}

export default class ChannelManager {
    // Internal reactive stores
    private channelsStore: Writable<Channel[]>;
    private currentUserChannelIdStore: Writable<string | null>;
    private isListeningStore: Writable<boolean>;
    private isLoadingStore: Writable<boolean>;
    private lastFetchTimeStore: Writable<number | null>;
    private errorStore: Writable<string | null>;
    private store: Store;
    private serverUrl: string;

    // Event management
    private eventUnlisten: (() => void) | null = null;
    private playerManager: PlayerManager;

    // Readonly exports for components
    public readonly channels: Readable<Channel[]>;
    public readonly currentUserChannelId: Readable<string | null>;
    public readonly isListening: Readable<boolean>;
    public readonly isLoading: Readable<boolean>;
    public readonly lastFetchTime: Readable<number | null>;
    public readonly error: Readable<string | null>;
    public readonly channelState: Readable<ChannelStoreState>;

    constructor(playerManager: PlayerManager, store: Store, serverUrl: string = '') {
        this.playerManager = playerManager;
        this.store = store;
        this.serverUrl = serverUrl;

        // Initialize internal stores
        this.channelsStore = writable<Channel[]>([]);
        this.currentUserChannelIdStore = writable<string | null>(null);
        this.isListeningStore = writable<boolean>(false);
        this.isLoadingStore = writable<boolean>(false);
        this.lastFetchTimeStore = writable<number | null>(null);
        this.errorStore = writable<string | null>(null);

        // Create readonly exports
        this.channels = { subscribe: this.channelsStore.subscribe };
        this.currentUserChannelId = { subscribe: this.currentUserChannelIdStore.subscribe };
        this.isListening = { subscribe: this.isListeningStore.subscribe };
        this.isLoading = { subscribe: this.isLoadingStore.subscribe };
        this.lastFetchTime = { subscribe: this.lastFetchTimeStore.subscribe };
        this.error = { subscribe: this.errorStore.subscribe };

        // Derived store for component convenience (matches current channelStore interface)
        this.channelState = derived(
            [this.channelsStore, this.currentUserChannelIdStore, this.isListeningStore, this.isLoadingStore, this.lastFetchTimeStore, this.errorStore],
            ([$channels, $currentUserChannelId, $isListening, $isLoading, $lastFetchTime, $error]) => ({
                channels: $channels,
                currentUserChannelId: $currentUserChannelId,
                isListening: $isListening,
                isLoading: $isLoading,
                lastFetchTime: $lastFetchTime,
                error: $error
            })
        );

        info(`ChannelManager: Initialized with server URL: ${serverUrl || 'none'}`);
    }

    // Helper function to get current user name from PlayerManager
    private getCurrentUserName(): string {
        // Get current user from PlayerManager instead of store
        const currentUserStore = this.playerManager.currentUser;
        let currentUser = '';
        const unsubscribe = currentUserStore.subscribe(value => {
            currentUser = value;
        });
        unsubscribe(); // Immediately unsubscribe since we just want the current value
        return currentUser;
    }

    private handleError(error: any): void {
        logError(`Channel manager error: ${error}`);
        this.errorStore.set(error.message || 'An unknown error occurred');
    }

    public clearError(): void {
        this.errorStore.set(null);
    }

    // Public API methods
    async initialize(): Promise<void> {
    }

    async fetchChannels(): Promise<void> {
        try {
            this.clearError();
            this.isLoadingStore.set(true);

            const channels = await invoke<Channel[]>('api_list_channels');

            this.channelsStore.set(channels);
            this.isLoadingStore.set(false);
            this.lastFetchTimeStore.set(Date.now());
        } catch (error) {
            logError(`Error fetching channels: ${error}`);
            this.isLoadingStore.set(false);
            this.handleError(error);
        }
    }

    async fetchChannel(channelId: string): Promise<Channel | null> {
        try {
            this.clearError();

            const channel = await invoke<Channel>('api_get_channel', { channelId });

            // Update or add the channel in the store
            this.channelsStore.update((channels: Channel[]) => {
                const existingIndex = channels.findIndex((c: Channel) => c.id === channelId);
                if (existingIndex >= 0) {
                    channels[existingIndex] = channel;
                    return [...channels];
                } else {
                    return [...channels, channel];
                }
            });

            return channel;
        } catch (error) {
            logError(`Failed to fetch channel ${channelId}: ${error}`);
            return null;
        }
    }

    async createChannel(name: string): Promise<string | null> {
        try {
            this.clearError();

            const channelId = await invoke<string>('api_create_channel', { name });

            // Refresh channels after creation
            await this.fetchChannels();

            return channelId;
        } catch (error) {
            this.handleError(error);
            return null;
        }
    }

    async deleteChannel(channelId: string): Promise<boolean> {
        try {
            this.clearError();

            const success = await invoke<boolean>('api_delete_channel', { channelId });

            if (success) {
                // Remove channel from local state immediately
                this.channelsStore.update((channels: Channel[]) => channels.filter((c: Channel) => c.id !== channelId));

                // Update current user channel if they were in the deleted channel
                const currentChannelId = get(this.currentUserChannelIdStore);
                if (currentChannelId === channelId) {
                    this.currentUserChannelIdStore.set(null);
                }
            }

            return success;
        } catch (error) {
            this.handleError(error);
            return false;
        }
    }

    async joinChannel(channelId: string, currentUser: string): Promise<boolean> {
        try {
            this.clearError();

            // If user is already in a channel, handle movement
            const currentChannelId = get(this.currentUserChannelIdStore);
            if (currentChannelId && currentChannelId !== channelId) {
                await this.handleChannelMovement(currentChannelId, channelId, currentUser);
            } else if (currentChannelId === channelId) {
                warn(`ChannelManager: User already in channel ${channelId}, skipping join`);
                return true;
            }

            const event: ChannelEvent = { event: "Join" as ChannelEvents };

            const success = await invoke<boolean>('api_channel_event', {
                channelId: channelId,
                event
            });

            if (success) {
                // Update local state optimistically
                this.currentUserChannelIdStore.set(channelId);
                this.channelsStore.update((channels: Channel[]) =>
                    channels.map((channel: Channel) => {
                        if (channel.id === channelId && !channel.players.includes(currentUser)) {
                            return {
                                ...channel,
                                players: [...channel.players, currentUser]
                            };
                        }
                        return channel;
                    })
                );

                // Add existing group members to PlayerManager
                await this.addExistingGroupMembers(channelId, currentUser);
            }

            return success;
        } catch (error: any) {
            // Check if the error is because the channel no longer exists
            if (error.message && error.message.includes('404')) {
                // Channel was deleted, refresh channels list
                await this.fetchChannels();
            }
            this.handleError(error);
            return false;
        }
    }

    async leaveChannel(channelId: string, currentUser: string): Promise<boolean> {
        try {
            this.clearError();

            const event: ChannelEvent = { event: "Leave" as ChannelEvents };

            const success = await invoke<boolean>('api_channel_event', {
                channelId: channelId,
                event
            });

            if (success) {
                // Update local state optimistically
                const currentChannelId = get(this.currentUserChannelIdStore);
                if (currentChannelId === channelId) {
                    this.currentUserChannelIdStore.set(null);
                }

                this.channelsStore.update((channels: Channel[]) =>
                    channels.map((channel: Channel) => {
                        if (channel.id === channelId) {
                            return {
                                ...channel,
                                players: channel.players.filter((p: string) => p !== currentUser)
                            };
                        }
                        return channel;
                    })
                );
            }

            return success;
        } catch (error) {
            this.handleError(error);
            return false;
        }
    }

    /**
     * Add existing group members when joining a channel
     */
    private async addExistingGroupMembers(channelId: string, currentUser: string): Promise<void> {
        if (!this.playerManager) {
            warn('ChannelManager: PlayerManager not available for adding group members');
            return;
        }

        const channels = get(this.channels);
        const channel = channels.find(c => c.id === channelId);

        if (!channel) {
            warn(`ChannelManager: Channel ${channelId} not found for adding existing members`);
            return;
        }

        for (const memberName of channel.players) {
            if (memberName !== currentUser) {
                try {
                    const success = await this.playerManager.addPlayerSource(memberName, 'Group');
                    if (!success) {
                        warn(`ChannelManager: Failed to add existing group member: ${memberName}`);
                    }
                } catch (err) {
                    logError(`ChannelManager: Error adding group member ${memberName}: ${err}`);
                }
            }
        }
    }

    /**
     * Remove all group members from PlayerManager (used when current user leaves channel)
     */
    private removeAllGroupMembers(memberNames: string[], currentUser: string, reason: string): void {
        if (!this.playerManager) {
            warn('ChannelManager: PlayerManager not available for removing group members');
            return;
        }

        memberNames.forEach(memberName => {
            if (memberName !== currentUser) {
                const success = this.playerManager.removePlayerSource(memberName, 'Group');
            }
        });
    }

    /**
     * Handle movement between channels - ensures proper cleanup and setup
     */
    private async handleChannelMovement(fromChannelId: string, toChannelId: string, currentUser: string): Promise<void> {
        // Get members from the old channel before leaving
        const channels = get(this.channels);
        const oldChannel = channels.find(c => c.id === fromChannelId);

        if (oldChannel && this.playerManager) {
            this.removeAllGroupMembers(oldChannel.players, currentUser, 'user moving to new channel');
        }

        // Leave the old channel
        await this.leaveChannel(fromChannelId, currentUser);
    }

    async startListening(): Promise<void> {
        if (get(this.isListening)) {
            return;
        }

        try {
            this.clearError();
            this.isListeningStore.set(true);

            const appWebview = getCurrentWebviewWindow();
            this.eventUnlisten = await appWebview.listen('channel_event', (event: any) => {
                this.handleChannelEvent(event);
            });
        } catch (error) {
            logError(`Failed to start channel event listener: ${error}`);
            this.handleError(error);
        }
    }

    stopListening(): void {
        if (this.eventUnlisten) {
            this.eventUnlisten();
            this.eventUnlisten = null;
        }
        this.isListeningStore.set(false);
    }

    private async handleChannelEvent(event: any): Promise<void> {
        const payload = event.payload;
        if (!payload) {
            logError("Channel event received with no payload");
            return;
        }

        const { event_type, channel_id, channel_name, creator, player_name } = payload;

        // Get current user name for group membership tracking
        const currentUser = this.getCurrentUserName();

        switch (event_type) {
            case 'create':
                // Fetch the new channel to get complete data
                await this.fetchChannel(channel_id);
                break;

            case 'delete':
                // Get channel members before deleting to clean up group memberships
                const channels = get(this.channels);
                const channelToDelete = channels.find(c => c.id === channel_id);

                if (channelToDelete && this.playerManager) {
                    this.removeAllGroupMembers(channelToDelete.players, currentUser, 'channel deleted');
                }

                // Remove the channel from our store
                this.channelsStore.update((channels: Channel[]) => channels.filter((c: Channel) => c.id !== channel_id));

                const currentChannelId = get(this.currentUserChannelIdStore);
                if (currentChannelId === channel_id) {
                    this.currentUserChannelIdStore.set(null);
                }
                break;

            case 'join':
                // Update the channel's player list
                this.channelsStore.update((channels: Channel[]) =>
                    channels.map((channel: Channel) => {
                        if (channel.id === channel_id && !channel.players.includes(player_name)) {
                            return { ...channel, players: [...channel.players, player_name] };
                        }
                        return channel;
                    })
                );

                // Add player to group membership if current user is in this channel
                if (currentUser && this.playerManager) {
                    const channels = get(this.channels);
                    const userChannel = channels.find(c => c.players.includes(currentUser));
                    if (userChannel && userChannel.id === channel_id && player_name !== currentUser) {
                        const success = await this.playerManager.addPlayerSource(player_name, 'Group');
                    }
                }
                break;

            case 'leave':
                // Check channel membership BEFORE removing from list
                let wasCurrentUserInChannel = false;
                let channelMembersBeforeLeave: string[] = [];
                if (currentUser) {
                    const channels = get(this.channels);
                    const userChannelBefore = channels.find(c => c.players.includes(currentUser));
                    wasCurrentUserInChannel = !!(userChannelBefore && userChannelBefore.id === channel_id);

                    // Capture channel members before any updates
                    const channelBeforeLeave = channels.find(c => c.id === channel_id);
                    if (channelBeforeLeave) {
                        channelMembersBeforeLeave = [...channelBeforeLeave.players];
                    }
                }

                // Remove player from the channel's player list
                this.channelsStore.update((channels: Channel[]) =>
                    channels.map((channel: Channel) => {
                        if (channel.id === channel_id) {
                            return { ...channel, players: channel.players.filter((p: string) => p !== player_name) };
                        }
                        return channel;
                    })
                );

                // Remove player from group membership if current user was in this channel
                if (currentUser && wasCurrentUserInChannel && this.playerManager) {
                    if (player_name === currentUser) {
                        this.removeAllGroupMembers(channelMembersBeforeLeave, currentUser, 'current user leaving channel');

                        // Clear current user's channel
                        this.currentUserChannelIdStore.set(null);
                    } else {
                        const success = this.playerManager.removePlayerSource(player_name, 'Group');
                    }
                }
                break;

            default:
                logError(`Unknown channel event type: ${event_type}`);
        }
    }

    // Cleanup method
    cleanup(): void {
        this.stopListening();
    }
}