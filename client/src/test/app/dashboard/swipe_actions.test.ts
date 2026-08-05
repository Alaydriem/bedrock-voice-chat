import { describe, expect, it } from 'vitest';
import { SwipeActions } from '$radial/core/controllers/SwipeActions';

const TRAY = 160;

describe('SwipeActions', () => {
    it('treats travel under the slop as a tap so the row still joins', () => {
        expect(SwipeActions.isSwipe(0)).toBe(false);
        expect(SwipeActions.isSwipe(-7)).toBe(false);
        expect(SwipeActions.isSwipe(-8)).toBe(true);
        expect(SwipeActions.isSwipe(8)).toBe(true);
    });

    it('follows a leftward drag one-for-one', () => {
        expect(SwipeActions.offset(-40, TRAY, false)).toBe(-40);
    });

    it('refuses to open rightward, where there is nothing behind the row', () => {
        expect(SwipeActions.offset(60, TRAY, false)).toBe(0);
    });

    it('stops a little past the tray rather than anywhere the finger goes', () => {
        expect(SwipeActions.offset(-9000, TRAY, false)).toBe(-(TRAY + SwipeActions.OVERSHOOT));
    });

    it('drags from where an already-open row is sitting', () => {
        expect(SwipeActions.offset(30, TRAY, true)).toBe(-130);
        expect(SwipeActions.offset(TRAY, TRAY, true)).toBe(0);
    });

    it('latches open only once the tray is substantially out', () => {
        expect(SwipeActions.latches(-63, TRAY)).toBe(false);
        expect(SwipeActions.latches(-64, TRAY)).toBe(true);
    });

    /**
     * A tray that has not been measured yet must not latch: `clientWidth` is zero before the
     * first layout, and a zero-width tray that latches leaves the row offset by nothing while
     * the parent believes a tray is open — a row that has stopped responding to taps.
     */
    it('never latches an unmeasured tray', () => {
        expect(SwipeActions.latches(-40, 0)).toBe(false);
    });

    it('rests flush when closed and against the tray when open', () => {
        expect(SwipeActions.resting(false, TRAY)).toBe(0);
        expect(SwipeActions.resting(true, TRAY)).toBe(-TRAY);
    });
});
