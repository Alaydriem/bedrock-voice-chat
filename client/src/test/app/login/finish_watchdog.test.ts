import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import FinishWatchdog from "../../../js/app/login/FinishWatchdog";

/**
 * The sign-in goes out to a browser and comes back as a deep link. This decides when that
 * link is never coming, and both of its mistakes are expensive: give up too early and a
 * working sign-in is reported as broken, never give up and the screen turns forever.
 *
 * Giving up too early is the one that was happening. The redemption is four sequential
 * upstream calls, each retried, and on mobile data it outran a fixed deadline.
 */

const { TIMEOUT_MS, EXTENSIONS } = FinishWatchdog;

function watchdog(options: {
  waiting?: () => boolean;
  inFlight?: () => Promise<boolean>;
}) {
  const onLost = vi.fn();
  const dog = new FinishWatchdog(
    options.waiting ?? (() => true),
    options.inFlight ?? (async () => false),
    onLost,
  );
  return { dog, onLost };
}

describe("FinishWatchdog", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("reports the sign-in lost when nothing came back", async () => {
    const { dog, onLost } = watchdog({});
    dog.start();

    await vi.advanceTimersByTimeAsync(TIMEOUT_MS);
    expect(onLost).toHaveBeenCalledOnce();
  });

  it("says nothing before its deadline", async () => {
    const { dog, onLost } = watchdog({});
    dog.start();

    await vi.advanceTimersByTimeAsync(TIMEOUT_MS - 1);
    expect(onLost).not.toHaveBeenCalled();
  });

  /**
   * The regression this exists for: a redemption still running when the deadline arrived was
   * declared lost, and the person retried a sign-in that was in the middle of succeeding.
   */
  it("waits again while a callback is still being processed", async () => {
    const { dog, onLost } = watchdog({ inFlight: async () => true });
    dog.start();

    await vi.advanceTimersByTimeAsync(TIMEOUT_MS);
    expect(onLost).not.toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(TIMEOUT_MS);
    expect(onLost).not.toHaveBeenCalled();
  });

  // Waiting on evidence with no ceiling is a spinner with extra steps. A handler that died
  // without clearing the pending entry would otherwise hold the screen open indefinitely.
  it("gives up after a bounded number of extensions", async () => {
    const { dog, onLost } = watchdog({ inFlight: async () => true });
    dog.start();

    await vi.advanceTimersByTimeAsync(TIMEOUT_MS * (EXTENSIONS + 1));
    expect(onLost).toHaveBeenCalledOnce();
  });

  it("fails on the first deadline when no callback ever arrived", async () => {
    const inFlight = vi.fn(async () => false);
    const { dog, onLost } = watchdog({ inFlight });
    dog.start();

    await vi.advanceTimersByTimeAsync(TIMEOUT_MS);
    expect(onLost).toHaveBeenCalledOnce();
    expect(inFlight).toHaveBeenCalledOnce();
  });

  it("stays quiet once the sign-in is no longer outstanding", async () => {
    const { dog, onLost } = watchdog({ waiting: () => false });
    dog.start();

    await vi.advanceTimersByTimeAsync(TIMEOUT_MS * (EXTENSIONS + 2));
    expect(onLost).not.toHaveBeenCalled();
  });

  /**
   * The evidence check awaits. A redemption that completes during it has already navigated,
   * and reporting the sign-in lost afterwards would replace a finished login with an error.
   */
  it("does not report a loss when the sign-in completes during its own check", async () => {
    let waiting = true;
    const onLost = vi.fn();
    const dog = new FinishWatchdog(
      () => waiting,
      async () => {
        waiting = false;
        return false;
      },
      onLost,
    );
    dog.start();

    await vi.advanceTimersByTimeAsync(TIMEOUT_MS);
    expect(onLost).not.toHaveBeenCalled();
  });

  it("stops when cancelled", async () => {
    const { dog, onLost } = watchdog({});
    dog.start();
    dog.cancel();

    await vi.advanceTimersByTimeAsync(TIMEOUT_MS * 2);
    expect(onLost).not.toHaveBeenCalled();
  });

  /**
   * A second attempt gets a full window. Extensions carried over from the previous one
   * would shorten the sign-in the user is actually waiting on.
   */
  it("restarts its allowance on a new attempt", async () => {
    let inFlight = true;
    const onLost = vi.fn();
    const dog = new FinishWatchdog(
      () => true,
      async () => inFlight,
      onLost,
    );

    dog.start();
    await vi.advanceTimersByTimeAsync(TIMEOUT_MS * EXTENSIONS);
    expect(onLost).not.toHaveBeenCalled();

    dog.start();
    inFlight = false;
    await vi.advanceTimersByTimeAsync(TIMEOUT_MS);
    expect(onLost).toHaveBeenCalledOnce();
  });
});
