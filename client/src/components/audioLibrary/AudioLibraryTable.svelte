<script lang="ts">
    import { onDestroy } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { error } from "@tauri-apps/plugin-log";
    import type { AudioFileResponse } from "../../js/bindings/AudioFileResponse";

    interface Props {
        files: AudioFileResponse[];
        total: number;
        page: number;
        pageSize: number;
        sortBy: string;
        sortOrder: string;
        searchQuery: string;
        canDelete: boolean;
        game?: string;
        onDelete: (fileId: string) => void;
        onPageChange: (page: number) => void;
        onSortChange: (sortBy: string, sortOrder: string) => void;
        onSearchChange: (query: string) => void;
    }

    let {
        files = [],
        total = 0,
        page = 0,
        pageSize = 20,
        sortBy = "created_at",
        sortOrder = "desc",
        searchQuery = "",
        canDelete = false,
        game,
        onDelete = () => {},
        onPageChange = () => {},
        onSortChange = () => {},
        onSearchChange = () => {},
    }: Props = $props();

    let confirmDeleteId: string | null = $state(null);
    let copiedId: string | null = $state(null);
    let searchTimeout: ReturnType<typeof setTimeout> | null = $state(null);
    let totalPages = $derived(Math.max(1, Math.ceil(total / pageSize)));

    let currentlyPlayingId: string | null = $state(null);
    let loadingFileId: string | null = $state(null);
    let audioElement: HTMLAudioElement | null = $state(null);

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

    function handleSearch(value: string) {
        if (searchTimeout) clearTimeout(searchTimeout);
        searchTimeout = setTimeout(() => {
            onSearchChange(value);
        }, 300);
    }

    function toggleSort(column: string) {
        if (sortBy === column) {
            onSortChange(column, sortOrder === "asc" ? "desc" : "asc");
        } else {
            onSortChange(column, "asc");
        }
    }

    function stopPlayback() {
        if (audioElement) {
            audioElement.pause();
            audioElement.src = "";
            audioElement = null;
        }
        currentlyPlayingId = null;
    }

    async function togglePlayback(fileId: string) {
        if (currentlyPlayingId === fileId) {
            stopPlayback();
            return;
        }

        stopPlayback();
        loadingFileId = fileId;

        try {
            const url = await invoke<string>("get_audio_stream_url", { fileId, game });
            audioElement = new Audio(url);
            audioElement.addEventListener("ended", () => stopPlayback());
            await audioElement.play();
            currentlyPlayingId = fileId;
        } catch (e) {
            error(`Failed to play audio file: ${e}`);
            stopPlayback();
        } finally {
            loadingFileId = null;
        }
    }

    onDestroy(() => stopPlayback());
</script>

<!-- Search -->
<div class="mb-4">
    <input
        type="text"
        class="form-input w-full rounded-lg border border-slate-300 bg-transparent px-3 py-2 placeholder:text-slate-400/70 dark:border-navy-450 dark:placeholder:text-navy-300/50"
        placeholder="Search by filename..."
        value={searchQuery}
        oninput={(e) => handleSearch((e.target as HTMLInputElement).value)}
    />
</div>

{#if files.length === 0 && searchQuery}
    <div class="flex flex-col items-center justify-center py-8 text-slate-400 dark:text-navy-300">
        <svg xmlns="http://www.w3.org/2000/svg" class="size-12 mb-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/>
        </svg>
        <p class="text-sm">No files matching "{searchQuery}"</p>
    </div>
{:else}
    <div class="is-scrollbar-hidden min-w-full overflow-x-auto">
        <table class="w-full text-left">
            <thead>
                <tr class="border-y border-transparent border-b-slate-200 dark:border-b-navy-500">
                    <th
                        class="whitespace-nowrap px-3 py-3 font-semibold uppercase text-slate-800 dark:text-navy-100 text-xs tracking-wider cursor-pointer select-none"
                        onclick={() => toggleSort("original_filename")}
                    >
                        <div class="flex items-center space-x-1">
                            <span>Name</span>
                            {#if sortBy === "original_filename"}
                                <svg xmlns="http://www.w3.org/2000/svg" class="size-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                    {#if sortOrder === "asc"}
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 15l7-7 7 7"/>
                                    {:else}
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/>
                                    {/if}
                                </svg>
                            {/if}
                        </div>
                    </th>
                    <th
                        class="whitespace-nowrap px-3 py-3 font-semibold uppercase text-slate-800 dark:text-navy-100 text-xs tracking-wider cursor-pointer select-none"
                        onclick={() => toggleSort("duration_ms")}
                    >
                        <div class="flex items-center space-x-1">
                            <span>Duration</span>
                            {#if sortBy === "duration_ms"}
                                <svg xmlns="http://www.w3.org/2000/svg" class="size-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                    {#if sortOrder === "asc"}
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 15l7-7 7 7"/>
                                    {:else}
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/>
                                    {/if}
                                </svg>
                            {/if}
                        </div>
                    </th>
                    <th
                        class="whitespace-nowrap px-3 py-3 font-semibold uppercase text-slate-800 dark:text-navy-100 text-xs tracking-wider cursor-pointer select-none"
                        onclick={() => toggleSort("file_size_bytes")}
                    >
                        <div class="flex items-center space-x-1">
                            <span>Size</span>
                            {#if sortBy === "file_size_bytes"}
                                <svg xmlns="http://www.w3.org/2000/svg" class="size-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                    {#if sortOrder === "asc"}
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 15l7-7 7 7"/>
                                    {:else}
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/>
                                    {/if}
                                </svg>
                            {/if}
                        </div>
                    </th>
                    <th
                        class="whitespace-nowrap px-3 py-3 font-semibold uppercase text-slate-800 dark:text-navy-100 text-xs tracking-wider cursor-pointer select-none"
                        onclick={() => toggleSort("created_at")}
                    >
                        <div class="flex items-center space-x-1">
                            <span>Date</span>
                            {#if sortBy === "created_at"}
                                <svg xmlns="http://www.w3.org/2000/svg" class="size-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                    {#if sortOrder === "asc"}
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 15l7-7 7 7"/>
                                    {:else}
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/>
                                    {/if}
                                </svg>
                            {/if}
                        </div>
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
                                class="btn size-7 rounded-full p-0 hover:bg-slate-300/20 focus:bg-slate-300/20 dark:hover:bg-navy-300/20 disabled:opacity-40 disabled:cursor-not-allowed"
                                class:text-primary={currentlyPlayingId === file.id}
                                class:text-slate-400={currentlyPlayingId !== file.id}
                                class:dark:text-navy-300={currentlyPlayingId !== file.id}
                                title={currentlyPlayingId === file.id ? "Stop" : "Preview"}
                                disabled={loadingFileId !== null && loadingFileId !== file.id}
                                onclick={() => togglePlayback(file.id)}
                            >
                                {#if loadingFileId === file.id}
                                    <svg xmlns="http://www.w3.org/2000/svg" class="size-4 animate-spin" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
                                    </svg>
                                {:else if currentlyPlayingId === file.id}
                                    <svg xmlns="http://www.w3.org/2000/svg" class="size-4" fill="currentColor" viewBox="0 0 24 24">
                                        <rect x="6" y="6" width="12" height="12" rx="1"/>
                                    </svg>
                                {:else}
                                    <svg xmlns="http://www.w3.org/2000/svg" class="size-4" fill="currentColor" viewBox="0 0 24 24">
                                        <path d="M8 5v14l11-7z"/>
                                    </svg>
                                {/if}
                            </button>
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

    <!-- Pagination -->
    {#if total > pageSize}
        <div class="flex items-center justify-between px-3 py-3 border-t border-slate-200 dark:border-navy-500">
            <span class="text-sm text-slate-500 dark:text-navy-300">
                Showing {page * pageSize + 1}–{Math.min((page + 1) * pageSize, total)} of {total}
            </span>
            <div class="flex items-center space-x-2">
                <button
                    class="btn size-8 rounded-full p-0 text-slate-400 hover:bg-slate-300/20 disabled:opacity-40 disabled:cursor-not-allowed dark:text-navy-300 dark:hover:bg-navy-300/20"
                    disabled={page === 0}
                    onclick={() => onPageChange(page - 1)}
                >
                    <svg xmlns="http://www.w3.org/2000/svg" class="size-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7"/>
                    </svg>
                </button>
                <span class="text-sm text-slate-600 dark:text-navy-200">
                    Page {page + 1} of {totalPages}
                </span>
                <button
                    class="btn size-8 rounded-full p-0 text-slate-400 hover:bg-slate-300/20 disabled:opacity-40 disabled:cursor-not-allowed dark:text-navy-300 dark:hover:bg-navy-300/20"
                    disabled={page >= totalPages - 1}
                    onclick={() => onPageChange(page + 1)}
                >
                    <svg xmlns="http://www.w3.org/2000/svg" class="size-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"/>
                    </svg>
                </button>
            </div>
        </div>
    {/if}
{/if}
