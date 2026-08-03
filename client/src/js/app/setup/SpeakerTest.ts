import { invoke } from '@tauri-apps/api/core';
import { warn } from '@tauri-apps/plugin-log';

/**
 * The speaker test on the setup device screen.
 *
 * The chime is generated in Rust and played through the output device the user selected,
 * not through the webview's default. That is the whole value of the test: a sound coming
 * out of whatever the platform happened to pick would confirm nothing about the device BVC
 * is about to use.
 *
 * The backend resolves only once the chime has finished, which is what lets the caller hold
 * the button disabled for exactly as long as it is playing rather than guessing at a
 * duration.
 */
export default class SpeakerTest {
    private playing = false;

    get isPlaying(): boolean {
        return this.playing;
    }

    /**
     * Play once. Returns whether it played; false means the device could not be opened,
     * which the screen reports rather than swallowing — a test that silently does nothing
     * is indistinguishable from a dead speaker, and that is the thing being diagnosed.
     */
    async play(): Promise<boolean> {
        if (this.playing) return true;
        this.playing = true;

        try {
            await invoke('test_output_device');
            return true;
        } catch (e) {
            await warn(`Speaker test failed: ${e}`);
            return false;
        } finally {
            this.playing = false;
        }
    }
}
