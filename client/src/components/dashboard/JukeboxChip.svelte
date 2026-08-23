<script lang="ts">
    import { onDestroy } from "svelte";
    import { I18n } from "$lib/i18n";
    import Icon from "$radial/components/Icon.svelte";
    import type { AudioSettingsManager } from "../../js/app/managers/settings/AudioSettingsManager";

    interface Props {
        audio: AudioSettingsManager;
        /** Whether jukebox frames are arriving, from the once-a-second runtime-state poll. */
        playing: boolean;
    }
    let { audio, playing }: Props = $props();

    let gain = $state(100);
    let muted = $state(false);
    let open = $state(false);

    const unsubs: Array<() => void> = [
        audio.jukeboxGain.subscribe((v) => (gain = v)),
        audio.jukeboxMuted.subscribe((v) => (muted = v)),
    ];

    onDestroy(() => {
        for (const off of unsubs) off();
    });

    function dismissOnOutsidePress(e: PointerEvent): void {
        if (!open) return;
        const target = e.target as HTMLElement | null;
        if (target?.closest(".rad-pop, .rad-header-btn--caret")) return;
        open = false;
    }
</script>

<!-- Two targets, one control. The body carries the action because muting is what somebody
     reaches for while music is playing; the caret carries the level, which is a decision rather
     than a reaction. On a phone there is no caret — a sub-target beside a header button this
     small does not survive touch — and the chip opens the sheet instead. -->
<button
    class="rad-header-btn"
    class:is-on={playing}
    aria-pressed={muted}
    aria-label={muted ? I18n.t("Unmute jukeboxes") : I18n.t("Mute jukeboxes")}
    onclick={() => void audio.handleJukeboxMutedChange(!muted)}
>
    <Icon name={muted ? "noteoff" : "note"} />
</button>

<button
    class="rad-header-btn rad-header-btn--caret rad-desk-only"
    aria-expanded={open}
    aria-label={I18n.t("Jukebox volume")}
    onclick={() => (open = !open)}
>
    <Icon name="chev" />
</button>

<!-- Dismissed by pressing away from it. Without this the popover is a mode: the only way out is
     the control that opened it. -->
<svelte:window onpointerdown={dismissOnOutsidePress} />

{#if open}
    <div class="rad-pop rad-desk-only">
        <div class="rad-pop__head">{I18n.t("Jukeboxes")}</div>
        <input
            class="rad-range"
            type="range"
            min="0"
            max="150"
            step="5"
            value={gain}
            disabled={muted}
            aria-label={I18n.t("Jukebox volume")}
            aria-valuetext="{gain}%"
            oninput={(e) =>
                void audio.handleJukeboxGainChange(
                    Number((e.target as HTMLInputElement).value),
                )}
        />
        <span class="rad-pop__value">{gain}%</span>
    </div>
{/if}
