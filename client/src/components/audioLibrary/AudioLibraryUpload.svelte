<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";

    interface Props {
        onUploadComplete: () => void;
        game?: string;
    }

    let { onUploadComplete = () => {}, game }: Props = $props();
    let isUploading = $state(false);
    let errorMessage = $state("");

    async function handleUpload() {
        errorMessage = "";

        try {
            // Use Tauri's dialog to pick a file
            const { open: dialogOpen } = await import("@tauri-apps/plugin-dialog");
            const selected = await dialogOpen({
                multiple: false,
                filters: [{
                    name: "Audio Files",
                    extensions: ["wav", "mp3", "ogg", "flac", "opus"]
                }]
            });

            if (!selected) return;

            const filePath = typeof selected === "string" ? selected : selected.path;
            if (!filePath) return;

            isUploading = true;

            await invoke("upload_audio_file", { filePath, game });
            onUploadComplete();
        } catch (e: any) {
            errorMessage = typeof e === "string" ? e : e?.message || "Upload failed";
        } finally {
            isUploading = false;
        }
    }
</script>

<div class="flex items-center space-x-3">
    <button
        class="btn bg-primary font-medium text-white hover:bg-primary-focus focus:bg-primary-focus active:bg-primary-focus/90 dark:bg-accent dark:hover:bg-accent-focus dark:focus:bg-accent-focus dark:active:bg-accent/90 disabled:opacity-50"
        onclick={handleUpload}
        disabled={isUploading}
    >
        {#if isUploading}
            <div class="spinner size-4 animate-spin rounded-full border-[3px] border-white/30 border-r-white mr-2"></div>
            Uploading...
        {:else}
            <svg xmlns="http://www.w3.org/2000/svg" class="size-4 mr-1.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"/>
            </svg>
            Upload Audio File
        {/if}
    </button>

    {#if errorMessage}
        <span class="text-sm text-error">{errorMessage}</span>
    {/if}
</div>
