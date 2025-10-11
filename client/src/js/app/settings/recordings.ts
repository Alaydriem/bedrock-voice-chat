import { info, error, warn } from '@tauri-apps/plugin-log';
import { invoke } from "@tauri-apps/api/core";
import { mount } from "svelte";

import RecordingsTable from '../../../components/recordings/RecordingsTable.svelte';
import LoadingState from '../../../components/recordings/LoadingState.svelte';
import EmptyState from '../../../components/recordings/EmptyState.svelte';

import type { RecordingSession } from '../../bindings/RecordingSession';
import type { SessionData } from '../../bindings/SessionData';

declare global {
  interface Window {
    App: any;
  }
}

export default class RecordingSettings {
    private recordings: RecordingSession[] = [];
    private isLoading: boolean = false;
    private currentComponent: any = null;

    constructor() {}

    async initialize(): Promise<boolean> {
        await this.loadRecordings();
        return true;
    }

    private async loadRecordings(): Promise<void> {
        this.isLoading = true;
        this.showLoadingState();

        try {
            this.recordings = await invoke<RecordingSession[]>("get_recording_sessions");
            info(`Loaded ${this.recordings.length} recording sessions`);
        } catch (e) {
            error(`Failed to load recordings: ${e}`);
            this.recordings = [];
        } finally {
            this.isLoading = false;
            this.renderContent();
        }
    }

    private showLoadingState(): void {
        this.clearCurrentComponent();
        const container = document.getElementById("recordings-table-container");
        if (container) {
            this.currentComponent = mount(LoadingState, {
                target: container
            });
        }
    }

    private renderContent(): void {
        this.clearCurrentComponent();
        const container = document.getElementById("recordings-table-container");
        if (!container) return;

        if (this.recordings.length === 0) {
            this.currentComponent = mount(EmptyState, {
                target: container
            });
        } else {
            this.currentComponent = mount(RecordingsTable, {
                target: container,
                props: {
                    recordings: this.recordings,
                    onExport: (sessionId: string, selectedPlayers: string[], withSpatial: boolean) => this.handleExport(sessionId, selectedPlayers, withSpatial),
                    onDelete: (sessionId: string) => this.handleDelete(sessionId)
                }
            });
        }
    }

    private clearCurrentComponent(): void {
        if (this.currentComponent) {
            this.currentComponent = null;
        }
        const container = document.getElementById("recordings-table-container");
        if (container) {
            container.innerHTML = '';
        }
    }

    private async handleExport(sessionId: string, selectedPlayers: string[], withSpatial: boolean): Promise<void> {
        try {
            info(`Exporting session ${sessionId} with ${selectedPlayers.length} participants (spatial: ${withSpatial})`);
            await invoke("export_recording", { 
                sessionId, 
                selectedPlayers, 
                spatial: withSpatial 
            });
        } catch (e) {
            error(`Failed to export recording: ${e}`);
        }
    }

    private async handleDelete(sessionId: string): Promise<void> {
        try {
            info(`Deleting session ${sessionId}`);
            await invoke("delete_recording_session", { sessionId });

            // Remove from local array and re-render
            this.recordings = this.recordings.filter(r => r.session_data.session_id !== sessionId);
            this.renderContent();
        } catch (e) {
            error(`Failed to delete recording: ${e}`);
        }
    }

    public async refresh(): Promise<void> {
        await this.loadRecordings();
        this.renderContent();
    }
}