<script lang="ts">
    import Hold from "../../js/app/stronghold.ts";
    import { Store } from '@tauri-apps/plugin-store';
    import { info, error, warn } from '@tauri-apps/plugin-log';
    import { invoke } from "@tauri-apps/api/core";

    const result = (async () => { 
        const secretStore = await Store.load('secrets.json', { autoSave: false });
        const password = await secretStore.get<{ value: string }>("stronghold_password");

        const store = await Store.load('store.json', { autoSave: false });
        const currentServer = await store.get<{ value: string }>("current_server");
        info(`Current server: ${currentServer?.value}`);

        if (password?.value) {
            const stronghold = await Hold.new("servers", password.value);
            const server = await stronghold.get(currentServer!.value);
            const s = JSON.stringify(server);
            //info(`Server: ${s}`);

            await invoke("update_current_player").then(() => {
                info("Current player updated");
            }).catch((e) => {
                error(`Error updating current player: ${e}`);
            });

            const inputDevice = await invoke("get_audio_device", {
                io: "InputDevice"
            }).then((device) => {
                return device;
            }).catch((e) => {
                error(`Error getting audio device: ${e}`);
            });

            await invoke("change_audio_device", {
                device: inputDevice
            }).then(() => {
                info("Audio Input Device changed");
            }).catch((e) => {
                error(`Error getting audio device: ${e}`);
            });

            const outputDevice = await invoke("get_audio_device", {
                io: "OutputDevice"
            }).then((device) => {
                return device;
            }).catch((e) => {
                error(`Error getting audio device: ${e}`);
            });

            await invoke("change_audio_device", {
                device: outputDevice
            }).then(() => {
                info("Audio Input Device changed");
            }).catch((e) => {
                error(`Error getting audio device: ${e}`);
            });

            
        }
    })();
</script>

This is the dashboard