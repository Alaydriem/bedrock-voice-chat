<script lang="ts">
    import { mount } from "svelte";
    import { info } from '@tauri-apps/plugin-log';
    import ParticipantAvatars from './ParticipantAvatars.svelte';
    import ParticipantSelector from './ParticipantSelector.svelte';
    import ExportDropdown from './ExportDropdown.svelte';
    import type { RecordingSession } from '../../js/bindings/RecordingSession';

    export let recordings: RecordingSession[] = [];
    export let onExport: (sessionId: string, selectedPlayers: string[], withSpatial: boolean) => Promise<void>;
    export let onDelete: (sessionId: string) => Promise<void>;

    // Track expanded sessions and selected participants
    let expandedSessions: string[] = [];
    let selectedParticipantsMap = new Map<string, string[]>();
    
    // Initialize selected participants for all recordings (all participants selected by default)
    $: {
        recordings.forEach(recording => {
            const sessionId = recording.session_data.session_id;
            if (!selectedParticipantsMap.has(sessionId)) {
                selectedParticipantsMap.set(sessionId, getAllParticipants(recording));
            }
        });
        info(`Initialized selectedParticipantsMap with ${selectedParticipantsMap.size} recordings`);
    }
    
    // Force reactivity tracking
    $: {
        info(`Reactive: expandedSessions changed to: ${JSON.stringify(expandedSessions)}`);
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

    function getAllParticipants(recording: RecordingSession): string[] {
        return [recording.session_data.emitter_player, ...recording.session_data.participants];
    }

    function toggleExpanded(sessionId: string) {
        info(`toggleExpanded called for sessionId: ${sessionId}`);
        info(`Current expandedSessions: ${JSON.stringify(expandedSessions)}`);
        
        if (expandedSessions.includes(sessionId)) {
            info(`Session is expanded, collapsing...`);
            expandedSessions = expandedSessions.filter(id => id !== sessionId);
        } else {
            info(`Session is collapsed, expanding...`);
            expandedSessions = [...expandedSessions, sessionId];
            // Initialize with all participants if not already set
            if (!selectedParticipantsMap.has(sessionId)) {
                const recording = recordings.find(r => r.session_data.session_id === sessionId);
                if (recording) {
                    selectedParticipantsMap.set(sessionId, getAllParticipants(recording));
                }
            }
        }
        
        info(`New expandedSessions: ${JSON.stringify(expandedSessions)}`);
    }

    function handleSelectionChange(sessionId: string, selectedParticipants: string[]) {
        selectedParticipantsMap.set(sessionId, selectedParticipants);
        selectedParticipantsMap = selectedParticipantsMap; // Trigger reactivity
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
                <tr class="border-b border-slate-200 hover:bg-slate-50 dark:border-navy-500 dark:hover:bg-navy-600">
                    <td class="whitespace-nowrap px-4 py-3 text-slate-600 dark:text-navy-100 lg:px-5">
                        <div class="flex items-center space-x-2">
                            <button
                                class="btn size-7 rounded-full p-0 hover:bg-slate-300/20 focus:bg-slate-300/20 active:bg-slate-300/25 dark:hover:bg-navy-300/20 dark:focus:bg-navy-300/20 dark:active:bg-navy-300/25"
                                on:click={() => toggleExpanded(recording.session_data.session_id)}
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
                            <div>
                                <p class="font-medium">{formatDate(recording.session_data.start_timestamp)}</p>
                                <p class="text-xs text-slate-400 dark:text-navy-300">
                                    {recording.session_data.duration_ms ? formatDuration(recording.session_data.duration_ms) : 'Unknown duration'}
                                </p>
                            </div>
                        </div>
                    </td>
                    <td class="whitespace-nowrap px-4 py-3 lg:px-5">
                        <ParticipantAvatars participants={getAllParticipants(recording)} maxVisible={4} />
                    </td>
                    <td class="whitespace-nowrap px-4 py-3 text-slate-600 dark:text-navy-100 lg:px-5">
                        <span class="font-medium">{recording.file_size_mb.toFixed(2)} MB</span>
                    </td>
                    <td class="whitespace-nowrap px-4 py-3 lg:px-5">
                        <ExportDropdown
                            sessionId={recording.session_data.session_id}
                            selectedParticipants={selectedParticipantsMap.get(recording.session_data.session_id) || getAllParticipants(recording)}
                            {onExport}
                            {onDelete}
                        />
                    </td>
                </tr>
                {#if expandedSessions.includes(recording.session_data.session_id)}
                    <tr class="border-b border-slate-200 bg-slate-50 dark:border-navy-500 dark:bg-navy-700">
                        <td colspan="4" class="px-0 py-0">
                            <div class="overflow-hidden transition-all duration-300 ease-in-out">
                                <ParticipantSelector
                                    participants={getAllParticipants(recording)}
                                    initialSelectedParticipants={selectedParticipantsMap.get(recording.session_data.session_id) || getAllParticipants(recording)}
                                    onSelectionChange={(selected) => handleSelectionChange(recording.session_data.session_id, selected)}
                                />
                            </div>
                        </td>
                    </tr>
                {/if}
            {/each}
        </tbody>
    </table>
</div>