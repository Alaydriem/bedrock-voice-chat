<script lang="ts">
  import { I18n } from "$lib/i18n";
    import { onMount } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { info, error } from "@tauri-apps/plugin-log";
    import Loader from "$radial/components/Loader.svelte";
    import SettingRow from "$radial/components/SettingRow.svelte";
    import PlatformDetector from "../../js/app/utils/PlatformDetector";
    import Analytics from "../../js/app/analytics";
    import { AudioDeviceIdentity } from "../../js/app/settings/AudioDeviceIdentity";
    import type { AudioDevice } from "../../js/bindings/AudioDevice";
    import type { AudioDeviceType } from "../../js/bindings/AudioDeviceType";

    /**
     * The input and output pickers, for the two screens that offer them.
     *
     * Mobile is asked first and answered by not rendering: Android and iOS route voice
     * themselves, so a picker there is a control that either lies or fights the system.
     */
    let isMobile = $state(false);
    let isLoading = $state(true);
    let failure = $state("");

    let inputDevices = $state<readonly AudioDevice[]>([]);
    let outputDevices = $state<readonly AudioDevice[]>([]);
    /** Identities rather than names: a name can belong to more than one device. */
    let selectedInput = $state("");
    let selectedOutput = $state("");

    onMount(async () => {
        isMobile = await new PlatformDetector().checkMobile();
        if (!isMobile) await loadDevices();
        isLoading = false;
    });

    async function loadDevices(): Promise<void> {
        try {
            const devices = await invoke<Record<string, AudioDevice[]>>("get_devices");
            const inputs: AudioDevice[] = [];
            const outputs: AudioDevice[] = [];

            for (const group of Object.values(devices)) {
                for (const device of group) {
                    (device.io === "InputDevice" ? inputs : outputs).push(device);
                }
            }

            const byName = (a: AudioDevice, b: AudioDevice) =>
                a.display_name.localeCompare(b.display_name);
            inputDevices = AudioDeviceIdentity.unique(inputs.sort(byName));
            outputDevices = AudioDeviceIdentity.unique(outputs.sort(byName));

            selectedInput = await current("InputDevice");
            selectedOutput = await current("OutputDevice");
        } catch (e) {
            failure = `${e}`;
            error(`Error loading audio devices: ${e}`);
        }
    }

    async function current(io: AudioDeviceType): Promise<string> {
        const device = await invoke<AudioDevice>("get_audio_device", { io });
        return AudioDeviceIdentity.keyOf(device);
    }

    async function choose(io: AudioDeviceType, key: string): Promise<void> {
        const pool = io === "InputDevice" ? inputDevices : outputDevices;
        const device = AudioDeviceIdentity.find(pool, key);
        if (!device) return;

        try {
            await invoke("set_audio_device", { device });
            info(`Audio device changed to ${device.display_name} for ${io}`);
            Analytics.track("AudioDeviceChanged", { device_type: io });
            await invoke("change_audio_device");
        } catch (e) {
            failure = `${e}`;
            error(`Error changing audio device: ${e}`);
        }
    }
</script>

{#if isMobile}
    <SettingRow
        label={I18n.t("Chosen by the system")}
        note={I18n.t("Your phone routes voice to whatever you last connected. Plug in a headset and it follows.")}
    />
{:else if isLoading}
    <div class="rad-empty" style="padding: 34px 20px">
        <Loader loading size={72} />
        <span class="rad-empty__note">{I18n.t("Getting your audio devices.")}</span>
    </div>
{:else}
    <SettingRow label={I18n.t("Input device")}>
        {#snippet control()}
            <select
                class="rad-select"
                aria-label={I18n.t("Input device")}
                value={selectedInput}
                onchange={(e) => {
                    selectedInput = (e.target as HTMLSelectElement).value;
                    void choose("InputDevice", selectedInput);
                }}
            >
                {#each inputDevices as device (AudioDeviceIdentity.keyOf(device))}
                    <option value={AudioDeviceIdentity.keyOf(device)}>
                        {device.display_name}
                    </option>
                {/each}
            </select>
        {/snippet}
    </SettingRow>

    <SettingRow label={I18n.t("Output device")}>
        {#snippet control()}
            <select
                class="rad-select"
                aria-label={I18n.t("Output device")}
                value={selectedOutput}
                onchange={(e) => {
                    selectedOutput = (e.target as HTMLSelectElement).value;
                    void choose("OutputDevice", selectedOutput);
                }}
            >
                {#each outputDevices as device (AudioDeviceIdentity.keyOf(device))}
                    <option value={AudioDeviceIdentity.keyOf(device)}>
                        {device.display_name}
                    </option>
                {/each}
            </select>
        {/snippet}
    </SettingRow>

    {#if failure}
        <div class="rad-callout rad-callout--warn" role="alert">
            <span><b>{I18n.t("The device list is incomplete.")}</b> {failure}</span>
        </div>
    {/if}
{/if}
