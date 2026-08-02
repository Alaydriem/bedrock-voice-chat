/**
 * Cycles a word in place, sizing its box to whichever word is showing.
 *
 * Widths are measured once after the fonts settle, then reused. Measuring every
 * frame would thrash layout for no benefit, and measuring before fonts load
 * gives you the fallback face's metrics.
 */

const STEP_MS = 2100;
const FADE_MS = 420;

function run(host: HTMLElement, hostIndex: number, still: boolean): void {
  const words = Array.from(host.querySelectorAll<HTMLElement>('[data-rotate-word]'));
  if (words.length === 0) return;

  const widths = words.map((w) => w.getBoundingClientRect().width);
  let current = 0;

  const size = (): void => {
    host.style.width = `${Math.ceil(widths[current]!)}px`;
  };
  size();

  if (words.length < 2 || still) return;

  const advance = (): void => {
    const previous = current;
    words[previous]!.classList.remove('is-on');
    words[previous]!.classList.add('is-out');
    words[previous]!.setAttribute('aria-hidden', 'true');

    current = (current + 1) % words.length;
    words[current]!.classList.remove('is-out');
    words[current]!.classList.add('is-on');
    words[current]!.removeAttribute('aria-hidden');
    size();

    // Clear the outgoing state once its transition finishes, so it is ready to
    // animate in again next lap.
    setTimeout(() => words[previous]!.classList.remove('is-out'), FADE_MS);
  };

  let timer = 0;
  const play = (): void => {
    if (timer) return;
    timer = window.setInterval(advance, STEP_MS);
  };
  const pause = (): void => {
    clearInterval(timer);
    timer = 0;
  };

  // Offset each rotator so two on a page do not flip in lockstep.
  setTimeout(play, hostIndex * 350);

  // Hold still while someone is reading it on purpose.
  host.addEventListener('pointerenter', pause);
  host.addEventListener('pointerleave', play);

  document.addEventListener('visibilitychange', () => {
    if (document.hidden) pause();
    else play();
  });
}

export function initRotators(): void {
  const hosts = Array.from(document.querySelectorAll<HTMLElement>('[data-rotate]'));
  if (hosts.length === 0) return;

  // Hands layout to the script: the first word stops sitting in normal flow, so
  // the explicit widths set above are what hold the box open.
  document.documentElement.classList.add('js-rot');

  const still = matchMedia('(prefers-reduced-motion: reduce)').matches;
  const start = (): void => hosts.forEach((h, i) => run(h, i, still));

  // Measuring before the font settles sizes every box to the fallback face.
  if (document.fonts && document.fonts.status !== 'loaded') {
    document.fonts.ready.then(start);
  } else {
    start();
  }
}
