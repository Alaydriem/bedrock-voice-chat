<script lang="ts">
    import { onDestroy } from "svelte";
    import { I18n } from "$lib/i18n";
    import SelfPill from "$radial/components/SelfPill.svelte";
    import type { SelfSnapshot } from "$radial/core/controllers/SelfState";
    import type { LevelSource } from "$radial/core/sources/LevelSource";
    import type { SelfController } from "../../js/app/dashboard/SelfController";
    import PlatformDetector from "../../js/app/utils/PlatformDetector";

    interface Props {
        controller: SelfController;
        selfState: SelfSnapshot;
        name: string;
        /** The group you are in, shown under your name. Empty for proximity only. */
        groupName?: string;
        /**
         * Passed in rather than called on the controller here, so the caller can burst at the
         * button it was pressed on. The centre has to be read before the state change, which
         * re-renders the pill and destroys the element the handler was handed.
         */
        onmute: (e: MouseEvent) => void;
        ondeafen: (e: MouseEvent) => void;
        onidentity: () => void;
        /** Renders the phone capsule instead of the desktop pill. */
        capsule?: boolean;
    }
    // Not `state`: a variable of that name in scope makes every `$state(...)` rune parse as
    // store access on it, so the rune stops working in this file.
    let {
        controller,
        selfState,
        name,
        groupName = "",
        onmute,
        ondeafen,
        onidentity,
        capsule = false,
    }: Props = $props();

    const source: LevelSource = $derived(controller.micSource);

    // Recording is a desktop control. Read synchronously rather than awaited, so the button
    // is never rendered for a frame on a platform that does not offer it.
    const desktop = !new PlatformDetector().mobile();

    /**
     * The elapsed timer, ticked here rather than by the controller.
     *
     * A second is the resolution the readout has, so a second is what it costs. Driving it
     * from the shared animation loop would repaint this text sixty times to change it once.
     */
    let recordTime = $state("00:00");
    let ticker: ReturnType<typeof setInterval> | null = null;

    $effect(() => {
        if (selfState.recording) {
            if (!ticker) {
                recordTime = controller.elapsed(performance.now());
                ticker = setInterval(() => {
                    recordTime = controller.elapsed(performance.now());
                }, 1000);
            }
        } else if (ticker) {
            clearInterval(ticker);
            ticker = null;
            recordTime = "00:00";
        }
    });

    onDestroy(() => {
        if (ticker) clearInterval(ticker);
    });
</script>

<SelfPill
    identityLabel={I18n.t("Profile and sign-out")}
    recordBlockedLabel={I18n.t("Recording is off on this server")}
    {name}
    state={selfState}
    {source}
    {groupName}
    {recordTime}
    showRecord={desktop}
    {capsule}
    {onmute}
    {ondeafen}
    onrecord={() => controller.pressRecord()}
    onhold={(down) => controller.hold(down)}
    {onidentity}
/>
