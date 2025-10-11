<script lang="ts">
    import { mount } from "svelte";
    import ParticipantAvatars from './ParticipantAvatars.svelte';
    import ExportDropdown from './ExportDropdown.svelte';
    import type { RecordingSession } from '../../js/bindings/RecordingSession';

    export let recordings: RecordingSession[] = [];
    export let onExport: (sessionId: string, withSpatial: boolean) => Promise<void>;
    export let onDelete: (sessionId: string) => Promise<void>;

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
                        <div>
                            <p class="font-medium">{formatDate(recording.session_data.start_timestamp)}</p>
                            <p class="text-xs text-slate-400 dark:text-navy-300">
                                {recording.session_data.duration_ms ? formatDuration(recording.session_data.duration_ms) : 'Unknown duration'}
                            </p>
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
                            {onExport}
                            {onDelete}
                        />
                    </td>
                </tr>
            {/each}
        </tbody>
    </table>
</div>