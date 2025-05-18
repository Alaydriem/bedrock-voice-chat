<script lang="ts">
    import Hold from "../../js/app/stronghold.ts";
    import { Store } from '@tauri-apps/plugin-store';
    import { info, error, warn } from '@tauri-apps/plugin-log';
    import { invoke } from "@tauri-apps/api/core";
    import type { AudioDevice } from "../../js/bindings/AudioDevice.ts";
    import type { AudioDeviceType } from "../../js/bindings/AudioDeviceType.ts";
    import type { LoginResponse } from "../../js/bindings/LoginResponse.ts";
    import type { Keypair } from "../../js/bindings/Keypair.ts";

    const result = (async () => { 
        const secretStore = await Store.load('secrets.json', { autoSave: false });
        const password = await secretStore.get<string>("stronghold_password");

        const store = await Store.load('store.json', { autoSave: false });
        const currentServer = await store.get<string>("current_server");
        info(`Current server: ${currentServer}`);

        if (password) {
            const stronghold = await Hold.new("servers", password);
            const server = await stronghold.get(currentServer);
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

            await invoke("change_audio_device", {
                device: outputDevice
            }).then(() => {
            }).catch((e) => {
                error(`Error getting audio device: ${e}`);
            });
            const c = await stronghold.get(currentServer)
                .then((data) => JSON.parse(data))
                .then((data) => {

                const keypair: Keypair = {
                    pk: data.keypair.pk,
                    sk: data.keypair.sk
                };

                const signature: Keypair = {
                    pk: data.signature.pk,
                    sk: data.signature.sk
                };

                const loginResponse: LoginResponse = {
                    gamerpic: data.gamerpic,
                    gamertag: data.gamertag,
                    quic_connect_string: data.quic_connect_string,
                    certificate_ca: data.certificate_ca,
                    certificate_key: data.certificate_key,
                    certificate: data.certificate,
                    keypair: keypair,
                    signature: signature,
                };

                return loginResponse;
            });

            await invoke("change_network_stream", {
                server: currentServer,
                data: c
            }).then(() => {
                info(`Changed network stream to ${currentServer}`);
            }).catch((e) => {
                error(`Error changing network stream: ${e}`);
            });

            const sleep = (ms: number): Promise<void> => {
                return new Promise(resolve => setTimeout(resolve, ms));
            };

            sleep(1000).then(() => {
                invoke("stop_audio_device", {
                    device: "InputDevice"
                }).then(() => {
                    info("stopped output device");
                }).catch((e) => {
                    error(`Error getting audio device: ${e}`);
                });

                invoke("change_audio_device", {
                    device: inputDevice,
                }).then(() => {
                    info("changed input device again");
                }).catch((e) => {
                    error(`Error getting audio device: ${e}`);
                });

            });

        }
    })();
</script>

This is the dashboard