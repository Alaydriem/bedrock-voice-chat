import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { warn } from '@tauri-apps/plugin-log';
import type { InputLevel } from '../../bindings/InputLevel';

/**
 * The microphone level on the setup device screen.
 *
 * Setup runs before the dashboard has started a session stream, so nothing is
 * capturing yet and `audio-input-level` would never fire. This asks the backend for a
 * capture-only stream: the same CPAL path, gate and processing core the session uses,
 * with no encoder and no network attached.
 *
 * The listener is registered before the stream is asked to start, so the first frames
 * are not dropped on the floor while the subscription is still resolving.
 */
export default class InputMeter {
    private unlisten: UnlistenFn | null = null;
    private running = false;

    constructor(private readonly onlevel: (level: InputLevel) => void) {}

    /**
     * Returns whether capture actually started. The caller needs the answer: a meter that
     * failed to start looks exactly like a microphone picking up nothing, and this screen
     * exists to tell those two apart.
     */
    async start(): Promise<boolean> {
        if (this.running) return true;
        this.running = true;

        this.unlisten = await listen<InputLevel>('audio-input-level', (event) =>
            this.onlevel(event.payload),
        );

        try {
            await invoke('start_input_meter');
            return true;
        } catch (e) {
            // A device that is missing or held exclusively by another application. The
            // rest of the screen still works, and the device picker is what the user came
            // here for, so this is reported rather than raised.
            await warn(`Could not start the input meter: ${e}`);
            return false;
        }
    }

    async stop(): Promise<void> {
        if (!this.running) return;
        this.running = false;

        try {
            await invoke('stop_input_meter');
        } catch (e) {
            await warn(`Could not stop the input meter: ${e}`);
        }

        this.unlisten?.();
        this.unlisten = null;
    }
}
