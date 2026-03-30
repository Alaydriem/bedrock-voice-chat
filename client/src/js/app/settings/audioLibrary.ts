import { info, error } from '@tauri-apps/plugin-log';
import { invoke } from "@tauri-apps/api/core";
import { mount, unmount } from "svelte";
import { Store } from "@tauri-apps/plugin-store";

import AudioLibraryTable from '../../../components/audioLibrary/AudioLibraryTable.svelte';
import AudioLibraryUpload from '../../../components/audioLibrary/AudioLibraryUpload.svelte';
import LoadingState from '../../../components/audioLibrary/LoadingState.svelte';
import EmptyState from '../../../components/audioLibrary/EmptyState.svelte';

import type { AudioFileResponse } from '../../bindings/AudioFileResponse';
import type { PaginatedResponse } from '../../bindings/PaginatedResponse';
import PlatformDetector from '../utils/PlatformDetector';

export default class AudioLibrarySettings {
    private files: AudioFileResponse[] = [];
    private totalFiles: number = 0;
    private currentPage: number = 0;
    private pageSize: number = 20;
    private sortBy: string = "created_at";
    private sortOrder: string = "desc";
    private searchQuery: string = "";
    private permissions: string[] = [];
    private isLoading: boolean = false;
    private currentComponent: any = null;
    private uploadComponent: any = null;
    private activeGame: string | null = null;
    private isMobile: boolean = false;

    constructor() {}

    async initialize(): Promise<boolean> {
        const platformDetector = new PlatformDetector();
        this.isMobile = await platformDetector.checkMobile();
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
                try {
                    const permStr = await invoke<string>("get_credential", {
                        server: currentServer,
                        key: "server_permissions"
                    });
                    if (permStr) {
                        const parsed = JSON.parse(permStr);
                        this.permissions = parsed.allowed || [];
                    }
                } catch {
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
            const result = await invoke<PaginatedResponse<AudioFileResponse>>("list_audio_files", {
                game: this.activeGame ?? undefined,
                query: {
                    page: this.currentPage,
                    page_size: this.pageSize,
                    sort_by: this.sortBy,
                    sort_order: this.sortOrder,
                    search: this.searchQuery || null,
                }
            });
            this.files = result.items;
            this.totalFiles = result.total;
            info(`Loaded ${this.files.length} of ${this.totalFiles} audio files`);
        } catch (e) {
            error(`Failed to load audio files: ${e}`);
            this.files = [];
            this.totalFiles = 0;
        } finally {
            this.isLoading = false;
            this.renderContent();
        }
    }

    async goToPage(page: number): Promise<void> {
        this.currentPage = page;
        await this.loadFiles();
    }

    async setSort(sortBy: string, sortOrder: string): Promise<void> {
        this.sortBy = sortBy;
        this.sortOrder = sortOrder;
        this.currentPage = 0;
        await this.loadFiles();
    }

    async setSearch(query: string): Promise<void> {
        this.searchQuery = query;
        this.currentPage = 0;
        await this.loadFiles();
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

        // Render upload section if permitted (desktop only - tauri-plugin-dialog is not available on mobile)
        const uploadContainer = document.getElementById("audio-library-upload-container");
        if (uploadContainer && this.hasPermission("audio_upload") && !this.isMobile) {
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

        if (this.files.length === 0 && !this.searchQuery) {
            this.currentComponent = mount(EmptyState, {
                target: container
            });
        } else {
            this.currentComponent = mount(AudioLibraryTable, {
                target: container,
                props: {
                    files: this.files,
                    total: this.totalFiles,
                    page: this.currentPage,
                    pageSize: this.pageSize,
                    sortBy: this.sortBy,
                    sortOrder: this.sortOrder,
                    searchQuery: this.searchQuery,
                    game: this.activeGame ?? undefined,
                    canDelete: this.hasPermission("audio_delete"),
                    onDelete: (fileId: string) => this.deleteFile(fileId),
                    onPageChange: (page: number) => this.goToPage(page),
                    onSortChange: (sortBy: string, sortOrder: string) => this.setSort(sortBy, sortOrder),
                    onSearchChange: (query: string) => this.setSearch(query),
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
