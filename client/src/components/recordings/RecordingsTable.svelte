<script lang="ts">
    import { info, debug } from '@tauri-apps/plugin-log';
    import { invoke } from '@tauri-apps/api/core';
    import ParticipantAvatars from './ParticipantAvatars.svelte';
    import ParticipantSelector from './ParticipantSelector.svelte';
    import ExportDropdown from './ExportDropdown.svelte';
    import type { RecordingSession } from '../../js/bindings/RecordingSession';
    import type { GamerpicResponse } from '../../js/bindings/GamerpicResponse';
    import ImageCache from '../../js/app/components/imageCache';
    import ImageCacheOptions from '../../js/app/components/imageCacheOptions';
    import GameNameUtils from '../../js/app/utils/GameNameUtils';

    const JUKEBOX_PREFIX = 'jukebox-';
    const GAMERPIC_TTL = 2592000;

    interface Props {
        recordings?: RecordingSession[];
        onExport: (sessionId: string, selectedPlayers: string[], withSpatial: boolean) => Promise<void>;
        onDelete: (sessionId: string) => Promise<void>;
    }

    let { recordings = [], onExport, onDelete }: Props = $props();

    let expandedSessions = $state<string[]>([]);
    let selectedParticipantsMap = $state<Record<string, string[]>>({});
    let jukeboxEnabledMap = $state<Record<string, boolean>>({});
    let gamerpicMap = $state<Record<string, string>>({});

    const imageCache = new ImageCache();
    const fetchInProgress = new Set<string>();

    $effect(() => {
        for (const recording of recordings) {
            const sessionId = recording.session_data.session_id;
            if (!(sessionId in selectedParticipantsMap)) {
                selectedParticipantsMap[sessionId] = getAllParticipants(recording);
            }

            for (const participant of getAllParticipants(recording)) {
                fetchGamepic(participant);
            }
        }
    });

    async function fetchGamepic(playerName: string): Promise<void> {
        if (playerName in gamerpicMap || fetchInProgress.has(playerName)) {
            return;
        }
        fetchInProgress.add(playerName);

        try {
            const game = GameNameUtils.extractGame(playerName);
            const gamertag = GameNameUtils.stripPrefix(playerName);
            const response = await invoke<GamerpicResponse>('api_get_player_gamerpic', { game, gamertag });

            if (response.gamerpic) {
                const options = new ImageCacheOptions(response.gamerpic, GAMERPIC_TTL);
                const dataUrl = await imageCache.getImage(options);
                gamerpicMap[playerName] = dataUrl;
            }
        } catch (err) {
            debug(`Failed to fetch gamerpic for ${playerName}: ${err}`);
        } finally {
            fetchInProgress.delete(playerName);
        }
    }

    function formatDate(timestamp: number): string {
        const date = new Date(timestamp);
        return date.toLocaleDateString('en-US', {
            year: 'numeric',
            month: 'short',
            day: 'numeric',
            hour: '2-digit',
            minute: '2-digit'
        });
    }

    function formatDuration(durationMs: number): string {
        const seconds = Math.floor(durationMs / 1000);
        const minutes = Math.floor(seconds / 60);
        const hours = Math.floor(minutes / 60);

        if (hours > 0) {
            return `${hours}h ${minutes % 60}m ${seconds % 60}s`;
        } else if (minutes > 0) {
            return `${minutes}m ${seconds % 60}s`;
        } else {
            return `${seconds}s`;
        }
    }

    function isJukeboxPlayer(name: string): boolean {
        return name.startsWith(JUKEBOX_PREFIX) || name.startsWith('jukebox::');
    }

    function getAllParticipants(recording: RecordingSession): string[] {
        return [recording.session_data.emitter_player, ...recording.session_data.participants]
            .filter(p => !isJukeboxPlayer(p));
    }

    function getJukeboxParticipants(recording: RecordingSession): string[] {
        return recording.session_data.jukebox_participants ?? [];
    }

    function hasJukeboxes(recording: RecordingSession): boolean {
        return getJukeboxParticipants(recording).length > 0;
    }

    function handleJukeboxToggle(sessionId: string, enabled: boolean) {
        jukeboxEnabledMap[sessionId] = enabled;
    }

    function getSelectedForExport(sessionId: string, recording: RecordingSession): string[] {
        const selected = selectedParticipantsMap[sessionId] || getAllParticipants(recording);
        if (jukeboxEnabledMap[sessionId]) {
            return [...selected, ...getJukeboxParticipants(recording)];
        }
        return selected;
    }

    function toggleExpanded(sessionId: string) {
        if (expandedSessions.includes(sessionId)) {
            expandedSessions = expandedSessions.filter(id => id !== sessionId);
        } else {
            expandedSessions = [...expandedSessions, sessionId];
            if (!(sessionId in selectedParticipantsMap)) {
                const recording = recordings.find(r => r.session_data.session_id === sessionId);
                if (recording) {
                    selectedParticipantsMap[sessionId] = getAllParticipants(recording);
                }
            }
        }
    }

    function handleSelectionChange(sessionId: string, selectedParticipants: string[]) {
        selectedParticipantsMap[sessionId] = selectedParticipants;
    }
</script>

<div class="is-scrollbar-hidden min-w-full">
    <table class="w-full text-left">
        <thead>
            <tr class="border-y border-slate-200 dark:border-navy-500">
                <th class="whitespace-nowrap bg-slate-200 px-4 py-3 font-semibold uppercase tracking-wide text-slate-800 dark:bg-navy-800 dark:text-navy-100 lg:px-5">
                    Date
                </th>
                <th class="whitespace-nowrap bg-slate-200 px-4 py-3 font-semibold uppercase tracking-wide text-slate-800 dark:bg-navy-800 dark:text-navy-100 lg:px-5">
                    Participants
                </th>
                <th class="whitespace-nowrap bg-slate-200 px-4 py-3 font-semibold uppercase tracking-wide text-slate-800 dark:bg-navy-800 dark:text-navy-100 lg:px-5">
                    Size
                </th>
                <th class="whitespace-nowrap bg-slate-200 px-4 py-3 font-semibold uppercase tracking-wide text-slate-800 dark:bg-navy-800 dark:text-navy-100 lg:px-5">
                    Actions
                </th>
            </tr>
        </thead>
        <tbody>
            {#each recordings as recording}
                <tr class="border-b border-slate-200 dark:border-navy-500
                    {recording.exportable
                        ? 'hover:bg-slate-50 dark:hover:bg-navy-600'
                        : 'opacity-50 bg-error/5 dark:bg-error/10 pointer-events-none select-none'}">
                    <td class="whitespace-nowrap px-4 py-3 lg:px-5">
                        <div class="flex items-center space-x-2">
                            {#if !recording.exportable}
                                <div class="flex size-7 items-center justify-center" title="Incompatible recording version — cannot export">
                                    <svg xmlns="http://www.w3.org/2000/svg" class="size-5 text-error" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/>
                                    </svg>
                                </div>
                            {:else}
                                <button
                                    class="btn size-7 rounded-full p-0 hover:bg-slate-300/20 focus:bg-slate-300/20 active:bg-slate-300/25 dark:hover:bg-navy-300/20 dark:focus:bg-navy-300/20 dark:active:bg-navy-300/25"
                                    onclick={() => toggleExpanded(recording.session_data.session_id)}
                                    aria-label="Toggle participants"
                                >
                                    <svg
                                        xmlns="http://www.w3.org/2000/svg"
                                        class="size-4 transition-transform duration-200 {expandedSessions.includes(recording.session_data.session_id) ? 'rotate-180' : ''}"
                                        fill="none"
                                        viewBox="0 0 24 24"
                                        stroke="currentColor"
                                    >
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
                                    </svg>
                                </button>
                            {/if}
                            <div class="{recording.exportable ? 'text-slate-600 dark:text-navy-100' : 'text-error/70 dark:text-error/60'}">
                                <p class="font-medium">{formatDate(recording.session_data.start_timestamp)}</p>
                                <p class="text-xs {recording.exportable ? 'text-slate-400 dark:text-navy-300' : 'text-error/50'}">
                                    {#if !recording.exportable}
                                        Incompatible version — export unavailable
                                    {:else}
                                        {recording.session_data.duration_ms ? formatDuration(recording.session_data.duration_ms) : 'Unknown duration'}
                                    {/if}
                                </p>
                            </div>
                        </div>
                    </td>
                    <td class="whitespace-nowrap px-4 py-3 lg:px-5">
                        <ParticipantAvatars participants={getAllParticipants(recording)} {gamerpicMap} maxVisible={4} />
                    </td>
                    <td class="whitespace-nowrap px-4 py-3 text-slate-600 dark:text-navy-100 lg:px-5">
                        <span class="font-medium">{recording.file_size_mb.toFixed(2)} MB</span>
                    </td>
                    <td class="whitespace-nowrap px-4 py-3 lg:px-5">
                        {#if recording.exportable}
                            <ExportDropdown
                                sessionId={recording.session_data.session_id}
                                selectedParticipants={getSelectedForExport(recording.session_data.session_id, recording)}
                                {onExport}
                                {onDelete}
                            />
                        {:else}
                            <button
                                class="btn size-7 rounded-full p-0 text-slate-400 hover:bg-error/10 hover:text-error focus:bg-error/10 pointer-events-auto"
                                onclick={() => onDelete(recording.session_data.session_id)}
                                title="Delete recording"
                            >
                                <svg xmlns="http://www.w3.org/2000/svg" class="size-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                                </svg>
                            </button>
                        {/if}
                    </td>
                </tr>
                {#if recording.exportable && expandedSessions.includes(recording.session_data.session_id)}
                    <tr class="border-b border-slate-200 bg-slate-50 dark:border-navy-500 dark:bg-navy-700">
                        <td colspan="4" class="px-0 py-0">
                            <div class="overflow-hidden transition-all duration-300 ease-in-out">
                                <ParticipantSelector
                                    participants={getAllParticipants(recording)}
                                    initialSelectedParticipants={selectedParticipantsMap[recording.session_data.session_id] || getAllParticipants(recording)}
                                    onSelectionChange={(selected) => handleSelectionChange(recording.session_data.session_id, selected)}
                                    hasJukeboxes={hasJukeboxes(recording)}
                                    jukeboxesEnabled={jukeboxEnabledMap[recording.session_data.session_id] || false}
                                    onJukeboxToggle={(enabled) => handleJukeboxToggle(recording.session_data.session_id, enabled)}
                                    {gamerpicMap}
                                />
                            </div>
                        </td>
                    </tr>
                {/if}
            {/each}
        </tbody>
    </table>
</div>
