import type { ParticipantLevel } from '../../bindings/ParticipantLevel';

/**
 * A quantised loudness step turned back into a meter level.
 *
 * The backend sends steps rather than amplitudes because a step stops changing while an
 * amplitude never does, and on Android a changed value is what costs a message. This undoes the
 * quantisation for display.
 *
 * It does not undo the *rate*. Steps arrive a couple of times a second at most, so a meter
 * driven straight off them moves in visible jumps — which is honest but ugly. Smoothing the gap
 * between them is a separate job and belongs to whatever drives the animation, not here.
 */
export class LevelSteps {
    /** Mirrors `ParticipantLevel::LOUDNESS_STEPS`. */
    static readonly STEPS = 8;

    /** A participant's reported state as a meter level in 0..1. */
    static toLevel(level: ParticipantLevel): number {
        if (!level.speaking) return 0;
        // Never zero while speaking. The backend already decided audio was passing, and a meter
        // reading empty over audible speech is the fault this whole path exists to avoid.
        const step = Math.max(1, Math.min(LevelSteps.STEPS, level.loudness));
        return step / LevelSteps.STEPS;
    }
}
