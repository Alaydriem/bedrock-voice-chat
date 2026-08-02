/**
 * Scroll reveal, replacing the GSAP + ScrollTrigger CDN pair the old site
 * loaded (about 70 KB of JavaScript from a third party, for a fade).
 *
 * Elements are visible until this runs, then the html.js-reveal class opts
 * them into the hidden-then-fade treatment. A script failure therefore leaves
 * a fully readable page rather than a blank one.
 */
export function initReveal(): void {
  const targets = document.querySelectorAll<HTMLElement>('[data-reveal]');
  if (targets.length === 0) return;

  if (matchMedia('(prefers-reduced-motion: reduce)').matches) return;

  document.documentElement.classList.add('js-reveal');

  const io = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue;
        const el = entry.target as HTMLElement;
        // Stagger siblings so a row of cards arrives in sequence.
        const index = Number(el.dataset.revealIndex ?? 0);
        el.style.setProperty('--reveal-delay', `${Math.min(index, 6) * 70}ms`);
        el.classList.add('is-in');
        io.unobserve(el);
      }
    },
    { rootMargin: '0px 0px -8% 0px', threshold: 0.08 }
  );

  for (const el of targets) io.observe(el);
}
