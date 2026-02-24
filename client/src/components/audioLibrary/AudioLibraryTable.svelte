<script lang="ts">
    import type { AudioFileResponse } from "../../js/app/settings/audioLibrary";

    interface Props {
        files: AudioFileResponse[];
        canDelete: boolean;
        onDelete: (fileId: string) => void;
    }

    let { files = [], canDelete = false, onDelete = () => {} }: Props = $props();
    let confirmDeleteId: string | null = $state(null);
    let copiedId: string | null = $state(null);

    function formatDuration(ms: number): string {
        const totalSeconds = Math.floor(ms / 1000);
        const minutes = Math.floor(totalSeconds / 60);
        const seconds = totalSeconds % 60;
        return `${minutes}:${seconds.toString().padStart(2, '0')}`;
    }

    function formatFileSize(bytes: number): string {
        if (bytes < 1024) return `${bytes} B`;
        if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
        return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    }

    function formatDate(timestamp: number): string {
        return new Date(timestamp * 1000).toLocaleDateString();
    }

    function handleDelete(fileId: string) {
        if (confirmDeleteId === fileId) {
            onDelete(fileId);
            confirmDeleteId = null;
        } else {
            confirmDeleteId = fileId;
        }
    }

    function cancelDelete() {
        confirmDeleteId = null;
    }

    async function copyId(fileId: string) {
        await navigator.clipboard.writeText(fileId);
        copiedId = fileId;
        setTimeout(() => { copiedId = null; }, 1500);
    }
</script>

<div class="is-scrollbar-hidden min-w-full overflow-x-auto">
    <table class="w-full text-left">
        <thead>
            <tr class="border-y border-transparent border-b-slate-200 dark:border-b-navy-500">
                <th class="whitespace-nowrap px-3 py-3 font-semibold uppercase text-slate-800 dark:text-navy-100 text-xs tracking-wider">
                    Name
                </th>
                <th class="whitespace-nowrap px-3 py-3 font-semibold uppercase text-slate-800 dark:text-navy-100 text-xs tracking-wider">
                    Duration
                </th>
                <th class="whitespace-nowrap px-3 py-3 font-semibold uppercase text-slate-800 dark:text-navy-100 text-xs tracking-wider">
                    Size
                </th>
                <th class="whitespace-nowrap px-3 py-3 font-semibold uppercase text-slate-800 dark:text-navy-100 text-xs tracking-wider">
                    Date
                </th>
                <th class="whitespace-nowrap px-3 py-3 font-semibold uppercase text-slate-800 dark:text-navy-100 text-xs tracking-wider">
                    Actions
                </th>
            </tr>
        </thead>
        <tbody>
            {#each files as file}
            <tr class="border-y border-transparent border-b-slate-200 dark:border-b-navy-500">
                <td class="whitespace-nowrap px-3 py-3 text-sm text-slate-600 dark:text-navy-200">
                    {file.original_filename}
                </td>
                <td class="whitespace-nowrap px-3 py-3 text-sm text-slate-600 dark:text-navy-200">
                    {formatDuration(file.duration_ms)}
                </td>
                <td class="whitespace-nowrap px-3 py-3 text-sm text-slate-600 dark:text-navy-200">
                    {formatFileSize(file.file_size_bytes)}
                </td>
                <td class="whitespace-nowrap px-3 py-3 text-sm text-slate-600 dark:text-navy-200">
                    {formatDate(file.created_at)}
                </td>
                <td class="whitespace-nowrap px-3 py-3">
                    <div class="flex items-center space-x-2">
                        <button
                            class="btn size-7 rounded-full p-0 text-slate-400 hover:bg-slate-300/20 focus:bg-slate-300/20 dark:text-navy-300 dark:hover:bg-navy-300/20"
                            title="Copy ID"
                            onclick={() => copyId(file.id)}
                        >
                            {#if copiedId === file.id}
                                <svg xmlns="http://www.w3.org/2000/svg" class="size-4 text-success" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/>
                                </svg>
                            {:else}
                                <svg xmlns="http://www.w3.org/2000/svg" class="size-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"/>
                                </svg>
                            {/if}
                        </button>
                        {#if canDelete}
                            {#if confirmDeleteId === file.id}
                                <button
                                    class="btn size-7 rounded-full bg-error/10 p-0 text-error hover:bg-error/20 focus:bg-error/20"
                                    onclick={() => handleDelete(file.id)}
                                >
                                    <svg xmlns="http://www.w3.org/2000/svg" class="size-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/>
                                    </svg>
                                </button>
                                <button
                                    class="btn size-7 rounded-full bg-slate-150 p-0 text-slate-500 hover:bg-slate-200 dark:bg-navy-500 dark:text-navy-200 dark:hover:bg-navy-450"
                                    onclick={cancelDelete}
                                >
                                    <svg xmlns="http://www.w3.org/2000/svg" class="size-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                                    </svg>
                                </button>
                            {:else}
                                <button
                                    class="btn size-7 rounded-full p-0 text-slate-400 hover:bg-slate-300/20 focus:bg-slate-300/20 dark:text-navy-300 dark:hover:bg-navy-300/20"
                                    onclick={() => handleDelete(file.id)}
                                >
                                    <svg xmlns="http://www.w3.org/2000/svg" class="size-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"/>
                                    </svg>
                                </button>
                            {/if}
                        {/if}
                    </div>
                </td>
            </tr>
            {/each}
        </tbody>
    </table>
</div>
