<script lang="ts">
    import Hold from "../../js/app/stronghold.ts";
    import { Store } from '@tauri-apps/plugin-store';
    import { info, error, warn } from '@tauri-apps/plugin-log';
    import { invoke } from "@tauri-apps/api/core";
    import type { AudioDevice } from "../../js/bindings/AudioDevice.ts";

    const result = (async () => { 
        const secretStore = await Store.load('secrets.json', { autoSave: false });
        const password = await secretStore.get<{ value: string }>("stronghold_password");

        const store = await Store.load('store.json', { autoSave: false });
        const currentServer = await store.get<{ value: string }>("current_server");
        info(`Current server: ${currentServer?.value}`);

        if (password?.value) {
            function sleep(ms: number): Promise<void> {
                return new Promise(resolve => setTimeout(resolve, ms));
            }

            const stronghold = await Hold.new("servers", password.value);
            const server = await stronghold.get(currentServer!.value);
            const s = JSON.stringify(server);
            
            await invoke("update_current_player").then(() => {

            }).catch((e) => {
                error(`Error updating current player: ${e}`);
            });

            const inputDevice = await invoke("get_audio_device", {
                io: "InputDevice"
            }).then(async (device) => device as AudioDevice)
            .then((device) => {
                return device;
            }).catch((e) => {
                error(`Error getting audio device: ${e}`);
            });

            await invoke("change_audio_device", {
                device: inputDevice
            }).then(() => {
            }).catch((e) => {
                error(`Error getting audio device: ${e}`);
            });

            const outputDevice = await invoke("get_audio_device", {
                io: "OutputDevice"
            }).then(async (device) => device as AudioDevice)
            .then((device) => {
                return device;
            }).catch((e) => {
                error(`Error getting audio device: ${e}`);
            });

            console.log(outputDevice);
            console.log(inputDevice);
            await invoke("change_audio_device", {
                device: outputDevice
            }).then(() => {
            }).catch((e) => {
                error(`Error getting audio device: ${e}`);
            });

            await sleep(5000);
            await invoke("stop_audio_device", { device: "InputDevice" }).then(() => {

            }).catch((e) => {
                error(`Error stopping audio device: ${e}`);
            });
            
            await sleep(5000);
            await invoke("stop_audio_device", { device: "OutputDevice" }).then(() => {

            }).catch((e) => {
                error(`Error stopping audio device: ${e}`);
            });
        }
    })();
</script>

This is the dashboard