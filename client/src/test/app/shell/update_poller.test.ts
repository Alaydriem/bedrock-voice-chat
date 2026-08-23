import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { UpdatePoller } from "../../../js/app/shell/UpdatePoller";
import type { UpdateStatus } from "../../../js/app/settings/UpdateStatus";

function stub() {
  const check = vi.fn(async () => {});
  return { check } as unknown as UpdateStatus & { check: typeof check };
}

describe("UpdatePoller", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  // The whole reason this moved off launch: nothing may compete with first paint.
  it("does not check before the first delay elapses", () => {
    const updates = stub();
    new UpdatePoller(updates, { firstDelayMs: 30_000, intervalMs: 60_000 }).start();

    vi.advanceTimersByTime(29_999);

    expect(updates.check).not.toHaveBeenCalled();
  });

  it("checks once the first delay elapses", () => {
    const updates = stub();
    new UpdatePoller(updates, { firstDelayMs: 30_000, intervalMs: 60_000 }).start();

    vi.advanceTimersByTime(30_000);

    expect(updates.check).toHaveBeenCalledTimes(1);
  });

  it("keeps checking on the interval after the first check", () => {
    const updates = stub();
    new UpdatePoller(updates, { firstDelayMs: 30_000, intervalMs: 60_000 }).start();

    vi.advanceTimersByTime(30_000 + 60_000 * 3);

    expect(updates.check).toHaveBeenCalledTimes(4);
  });

  // A poller that outlived its screen would keep checking for the life of the process.
  it("stops checking once stopped", () => {
    const updates = stub();
    const poller = new UpdatePoller(updates, { firstDelayMs: 30_000, intervalMs: 60_000 });
    poller.start();

    vi.advanceTimersByTime(30_000);
    poller.stop();
    vi.advanceTimersByTime(60_000 * 5);

    expect(updates.check).toHaveBeenCalledTimes(1);
  });

  it("ignores a second start rather than doubling the cadence", () => {
    const updates = stub();
    const poller = new UpdatePoller(updates, { firstDelayMs: 30_000, intervalMs: 60_000 });
    poller.start();
    poller.start();

    vi.advanceTimersByTime(30_000);

    expect(updates.check).toHaveBeenCalledTimes(1);
  });

  // stop() before the first delay has to cancel the pending timeout, not just the interval
  // that has not been created yet.
  it("cancels a pending first check when stopped early", () => {
    const updates = stub();
    const poller = new UpdatePoller(updates, { firstDelayMs: 30_000, intervalMs: 60_000 });
    poller.start();

    poller.stop();
    vi.advanceTimersByTime(30_000 * 2);

    expect(updates.check).not.toHaveBeenCalled();
  });
});
