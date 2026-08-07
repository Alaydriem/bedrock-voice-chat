import type { NoiseGateSettings } from "../../bindings/NoiseGateSettings";

export type { NoiseGateSettings };

export interface GateKnob {
    readonly id: keyof NoiseGateSettings;
    readonly label: string;
    readonly note: string;
    readonly unit: string;
    readonly min: number;
    readonly max: number;
}

/**
 * The noise gate's five numbers.
 *
 * The close threshold is held below the open threshold. A gate configured the other way
 * round never shuts, and nothing in the audio path refuses it.
 */
export class NoiseGateModel {
    /**
     * The one set of defaults on this side of the boundary: what a launch seeds into the
     * store and what Reset restores are the same numbers, and `NoiseGateSettings::default`
     * in `common` carries them too. Two literals drifted apart once, and a gate the user
     * never touched then behaved differently from one they had reset.
     */
    static readonly DEFAULTS: NoiseGateSettings = {
        open_threshold: -40,
        close_threshold: -50,
        release_rate: 100,
        attack_rate: 10,
        hold_time: 50,
    };

    /** How far below the open threshold the close threshold is held. */
    static readonly HYSTERESIS = 2;

    static readonly KNOBS: readonly GateKnob[] = [
        {
            id: "open_threshold",
            label: "Opens above",
            note: "Loud enough to count as speech.",
            unit: "dBFS",
            min: -96,
            max: 0,
        },
        {
            id: "close_threshold",
            label: "Closes below",
            note: "Held under the level that opens it, so a steady voice at the edge does not stutter.",
            unit: "dBFS",
            min: -96,
            max: 0,
        },
        {
            id: "attack_rate",
            label: "Attack",
            note: "Slow enough to hear as a fade, fast enough not to clip a word.",
            unit: "ms",
            min: 0,
            max: 250,
        },
        {
            id: "hold_time",
            label: "Hold",
            note: "Held open through the gap between two words.",
            unit: "ms",
            min: 0,
            max: 250,
        },
        {
            id: "release_rate",
            label: "Release",
            note: "How long it stays open after you stop talking.",
            unit: "ms",
            min: 0,
            max: 250,
        },
    ];

    /** Applies one change. The threshold that did not move is the one that yields. */
    static apply(
        settings: NoiseGateSettings,
        id: keyof NoiseGateSettings,
        value: number,
    ): NoiseGateSettings {
        const next = { ...settings, [id]: value };

        if (id === "open_threshold" && next.close_threshold > value - this.HYSTERESIS) {
            next.close_threshold = value - this.HYSTERESIS;
        }
        if (id === "close_threshold" && value > next.open_threshold - this.HYSTERESIS) {
            next.open_threshold = value + this.HYSTERESIS;
        }

        return this.clampToRange(next);
    }

    /** Nothing may leave the range its own control offers. */
    static clampToRange(settings: NoiseGateSettings): NoiseGateSettings {
        const clamped = { ...settings };
        for (const knob of this.KNOBS) {
            clamped[knob.id] = Math.min(knob.max, Math.max(knob.min, clamped[knob.id]));
        }
        return clamped;
    }

    /** What a knob's readout says. */
    static format(knob: GateKnob, value: number): string {
        return `${Math.round(value)} ${knob.unit}`;
    }

    /** Fills in anything a stored config is missing, so a partial object still loads. */
    static hydrate(stored: Partial<NoiseGateSettings> | null | undefined): NoiseGateSettings {
        return this.clampToRange({ ...this.DEFAULTS, ...(stored ?? {}) });
    }
}
