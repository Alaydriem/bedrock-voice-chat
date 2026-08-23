/**
 * Decides when a sign-in that went out to the browser has been lost.
 *
 * The app hands off to an external browser and gets the result back as a deep link. Nothing
 * guarantees that link arrives — the intent can be dropped, the browser can be dismissed,
 * the user can wander off — so someone has to declare the attempt lost rather than leave a
 * spinner turning. This does, on a deadline.
 *
 * The deadline alone was the problem: it is a guess about how long a redemption takes, and
 * on a phone the guess is wrong. The exchange is four sequential upstream calls — Microsoft,
 * Xbox Live, XSTS, Minecraft Services — each retried, over mobile data. A sign-in that would
 * have succeeded was declared lost, and the person retried something that was working.
 *
 * So the deadline now asks whether a callback is actually being processed before it fails,
 * and waits again if one is. Bounded, because "wait for evidence" with no ceiling is just a
 * spinner with extra steps: the pending entry is cleared by the handler, and a handler that
 * died without clearing it would otherwise hold the screen open forever.
 */
export default class FinishWatchdog {
  static readonly TIMEOUT_MS = 30000;

  /** Further windows granted while a callback is demonstrably still in progress. */
  static readonly EXTENSIONS = 3;

  private timer: ReturnType<typeof setTimeout> | null = null;
  private extensions = 0;

  /**
   * @param stillWaiting whether the sign-in is still outstanding. Asked before the verdict
   *   and again after the evidence check, which awaits.
   * @param inFlight whether a callback has arrived and not yet been disposed of.
   * @param onLost called once, when the attempt is judged lost.
   * @param log where a granted extension is recorded, so a slow sign-in is visible in a log
   *   rather than looking like the deadline silently not working.
   */
  constructor(
    private readonly stillWaiting: () => boolean,
    private readonly inFlight: () => Promise<boolean>,
    private readonly onLost: () => void,
    private readonly log: (message: string) => void = () => {},
  ) {}

  start(): void {
    this.cancel();
    this.arm();
  }

  cancel(): void {
    this.extensions = 0;
    if (this.timer !== null) {
      clearTimeout(this.timer);
      this.timer = null;
    }
  }

  private arm(): void {
    this.timer = setTimeout(() => {
      void this.deadlineReached();
    }, FinishWatchdog.TIMEOUT_MS);
  }

  private async deadlineReached(): Promise<void> {
    this.timer = null;
    if (!this.stillWaiting()) return;

    if (this.extensions < FinishWatchdog.EXTENSIONS && (await this.inFlight())) {
      this.extensions += 1;
      this.log(
        `Login: auth callback still in progress, waiting (${this.extensions}/${FinishWatchdog.EXTENSIONS})`,
      );
      this.arm();
      return;
    }

    // Asked again because the evidence check awaited: a redemption that finished while it
    // did has already moved the sign-in on, and failing it now would replace a completed
    // login with an error.
    if (!this.stillWaiting()) return;

    this.onLost();
  }
}
