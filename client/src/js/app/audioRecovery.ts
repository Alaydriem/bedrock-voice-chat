import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { invoke } from "@tauri-apps/api/core";
import { info, warn, error } from '@tauri-apps/plugin-log';
import { mount } from "svelte";
import Notification from "../../components/events/Notification.svelte";

/**
 * Audio stream recovery event payload from the Rust backend
 */
interface AudioStreamRecoveryPayload {
    device_type: 'InputDevice' | 'OutputDevice';
    error: string;
}

/**
 * Sets up audio stream recovery handling.
 * Listens for audio-stream-recovery events from the backend and attempts to restart the stream.
 *
 * @returns A cleanup function to remove the event listener
 */
export async function setupAudioRecovery(): Promise<() => void> {
    const appWebview = getCurrentWebviewWindow();

    const unlisten = await appWebview.listen<AudioStreamRecoveryPayload>('audio-stream-recovery', async (event) => {
        const { device_type, error: streamError } = event.payload;

        warn(`Audio stream error on ${device_type}: ${streamError}`);

        // Show notification to user
        showRecoveryNotification(
            "Audio Device Error",
            `Audio stream error detected. Attempting to recover...`,
            "warning"
        );

        // Brief delay before restart attempt to allow device state to settle
        await new Promise(resolve => setTimeout(resolve, 500));

        try {
            info(`Attempting to restart ${device_type} stream...`);
            await invoke('restart_audio_stream', { device: device_type });
            info(`Audio stream ${device_type} recovered successfully`);

            showRecoveryNotification(
                "Audio Recovered",
                `Audio stream recovered successfully.`,
                "success"
            );
        } catch (e) {
            error(`Audio recovery failed for ${device_type}: ${e}`);

            showRecoveryNotification(
                "Audio Recovery Failed",
                `Could not recover audio stream. Try changing your audio device in settings.`,
                "error"
            );
        }
    });

    info("Audio stream recovery handler initialized");
    return unlisten;
}

/**
 * Shows a notification to the user about audio recovery status
 */
function showRecoveryNotification(title: string, body: string, level: 'info' | 'warning' | 'error' | 'success'): void {
    const container = document.querySelector("#notification-container");
    if (container) {
        mount(Notification, {
            target: container,
            props: {
                title,
                body,
                level
            }
        });
    }
}
