import { info, error } from '@tauri-apps/plugin-log';
import { invoke } from "@tauri-apps/api/core";
import { mount, unmount } from "svelte";
import { Store } from "@tauri-apps/plugin-store";

import AudioLibraryTable from '../../../components/audioLibrary/AudioLibraryTable.svelte';
import AudioLibraryUpload from '../../../components/audioLibrary/AudioLibraryUpload.svelte';
import LoadingState from '../../../components/audioLibrary/LoadingState.svelte';
import EmptyState from '../../../components/audioLibrary/EmptyState.svelte';

import Keyring from '../keyring';

export interface AudioFileResponse {
    id: string;
    uploader_id: number;
    original_filename: string;
    duration_ms: number;
    file_size_bytes: number;
    game: string;
    created_at: number;
}

export interface AuthStateResponse {
    server_permissions: {
        allowed: string[];
    };
}

export default class AudioLibrarySettings {
    private files: AudioFileResponse[] = [];
    private permissions: string[] = [];
    private isLoading: boolean = false;
    private currentComponent: any = null;
    private uploadComponent: any = null;
    private activeGame: string | null = null;

    constructor() {}

    async initialize(): Promise<boolean> {
        const store = await Store.load("store.json", { autoSave: false, defaults: {} });
        this.activeGame = await store.get<string>("active_game") ?? null;
        await this.loadPermissions();
        await this.loadFiles();
        return true;
    }

    private async loadPermissions(): Promise<void> {
        try {
            const store = await Store.load("store.json", { autoSave: false, defaults: {} });
            const currentServer = await store.get<string>("current_server");

            if (currentServer) {
                const keyring = await Keyring.new("servers");
                await keyring.setServer(currentServer);

                try {
                    const permStr = await keyring.get("server_permissions");
                    if (permStr && typeof permStr === "string") {
                        const parsed = JSON.parse(permStr);
                        this.permissions = parsed.allowed || [];
                    }
                } catch {
                    // No permissions stored yet, use defaults
                    this.permissions = ["audio_upload", "audio_delete"];
                }
            }
        } catch (e) {
            error(`Failed to load permissions: ${e}`);
            this.permissions = [];
        }
    }

    hasPermission(perm: string): boolean {
        return this.permissions.includes(perm);
    }

    private async loadFiles(): Promise<void> {
        this.isLoading = true;
        this.showLoadingState();

        try {
            this.files = await invoke<AudioFileResponse[]>("list_audio_files", { game: this.activeGame ?? undefined });
            info(`Loaded ${this.files.length} audio files`);
        } catch (e) {
            error(`Failed to load audio files: ${e}`);
            this.files = [];
        } finally {
            this.isLoading = false;
            this.renderContent();
        }
    }

    private showLoadingState(): void {
        this.clearCurrentComponent();
        const container = document.getElementById("audio-library-table-container");
        if (container) {
            this.currentComponent = mount(LoadingState, {
                target: container
            });
        }
    }

    private renderContent(): void {
        this.clearCurrentComponent();

        // Render upload section if permitted
        const uploadContainer = document.getElementById("audio-library-upload-container");
        if (uploadContainer && this.hasPermission("audio_upload")) {
            if (this.uploadComponent) {
                unmount(this.uploadComponent);
            }
            this.uploadComponent = mount(AudioLibraryUpload, {
                target: uploadContainer,
                props: {
                    onUploadComplete: () => this.loadFiles(),
                    game: this.activeGame ?? undefined
                }
            });
        }

        // Render table or empty state
        const container = document.getElementById("audio-library-table-container");
        if (!container) return;

        if (this.files.length === 0) {
            this.currentComponent = mount(EmptyState, {
                target: container
            });
        } else {
            this.currentComponent = mount(AudioLibraryTable, {
                target: container,
                props: {
                    files: this.files,
                    canDelete: this.hasPermission("audio_delete"),
                    onDelete: (fileId: string) => this.deleteFile(fileId)
                }
            });
        }
    }

    async deleteFile(fileId: string): Promise<void> {
        try {
            await invoke<boolean>("delete_audio_file", { fileId, game: this.activeGame ?? undefined });
            info(`Deleted audio file: ${fileId}`);
            await this.loadFiles();
        } catch (e) {
            error(`Failed to delete audio file: ${e}`);
        }
    }

    private clearCurrentComponent(): void {
        if (this.currentComponent) {
            try {
                unmount(this.currentComponent);
            } catch {
                // Component may already be unmounted
            }
            this.currentComponent = null;
        }
    }
}
