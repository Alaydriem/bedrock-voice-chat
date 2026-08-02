import {
  anchorFor,
  AUDIENCE_ROTATE_MS,
  AUDIENCES,
  DEFAULT_AUDIENCE,
  isAudience,
  type Audience,
} from './site';

/**
 * The audience switcher.
 *
 * Every variant is already in the DOM; this only moves an attribute on <html>
 * and CSS does the rest (see the audience block in styles/base.css). That
 * means switching costs no layout thrash, no fetch, and no loading state, and
 * a crawler sees all three tracks.
 *
 * Precedence: ?you= in the URL, then the last choice from localStorage, then
 * the default. The URL wins so a link can be aimed at an audience — an ad for
 * server admins can point at /?you=operator and land on the right copy.
 */

const STORAGE_KEY = 'bvc:audience';
const PARAM = 'you';

export const AUDIENCE_CHANGE_EVENT = 'bvc:audiencechange';

export function readAudience(): Audience {
  const fromUrl = new URLSearchParams(location.search).get(PARAM);
  if (isAudience(fromUrl)) return fromUrl;

  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (isAudience(stored)) return stored;
  } catch {
    // Private browsing or blocked storage. The default is fine.
  }

  return DEFAULT_AUDIENCE;
}

export function applyAudience(next: Audience, persist = true): void {
  document.documentElement.dataset.audience = next;

  for (const btn of document.querySelectorAll<HTMLElement>('[data-audience-pick]')) {
    btn.setAttribute('aria-pressed', String(btn.dataset.audiencePick === next));
  }

  if (persist) {
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {
      // Nothing to do; the choice just will not survive a reload.
    }
    const url = new URL(location.href);
    url.searchParams.set(PARAM, next);
    history.replaceState(null, '', url);
  }

  document.dispatchEvent(new CustomEvent(AUDIENCE_CHANGE_EVENT, { detail: next }));
}

/**
 * Cycles the hero through all three tracks so a visitor who does not click
 * anything still sees that BVC answers three different questions.
 *
 * Deliberately fragile: it stops for good the moment someone picks a track,
 * and never starts at all if they arrived on a ?you= link or have already
 * chosen on a previous visit. Rotating the headline out from under someone who
 * has told us who they are would be a bug, not a feature.
 */
function startRotation(): void {
  if (matchMedia('(prefers-reduced-motion: reduce)').matches) return;

  // An explicit choice — via URL or a previous visit — outranks the carousel.
  const explicit = new URLSearchParams(location.search).has('you');
  let stored = false;
  try {
    stored = isAudience(localStorage.getItem('bvc:audience'));
  } catch {
    stored = false;
  }
  if (explicit || stored) return;

  let index = 0;
  let timer = window.setInterval(step, AUDIENCE_ROTATE_MS);
  let stopped = false;

  function step(): void {
    index = (index + 1) % AUDIENCES.length;
    // persist:false — an automatic rotation is not a choice, so it must not be
    // written to storage or the URL.
    applyAudience(AUDIENCES[index]!, false);
  }

  function stop(): void {
    if (stopped) return;
    stopped = true;
    clearInterval(timer);
    timer = 0;
  }

  // Any deliberate interaction with the control ends the rotation.
  for (const el of document.querySelectorAll('[data-audience-rotates]')) {
    el.addEventListener('pointerdown', stop);
    el.addEventListener('focusin', stop);
  }
  document.addEventListener('bvc:audiencepicked', stop);

  document.addEventListener('visibilitychange', () => {
    if (stopped) return;
    if (document.hidden) {
      clearInterval(timer);
      timer = 0;
    } else if (!timer) {
      timer = window.setInterval(step, AUDIENCE_ROTATE_MS);
    }
  });
}

/**
 * Take the reader to the section they just identified with.
 *
 * The switcher lives in a full-height hero, so switching alone changed a
 * headline the reader was already looking at and nothing they could see below
 * it. Scrolling is what makes the click feel like it did something, and it also
 * means the control never has to hide anything to prove it works.
 */
function scrollToAudience(id: Audience): void {
  const target = document.getElementById(anchorFor(id));
  if (!target) return;
  const still = matchMedia('(prefers-reduced-motion: reduce)').matches;
  target.scrollIntoView({ behavior: still ? 'auto' : 'smooth', block: 'start' });
}

export function initAudience(): void {
  // The inline head script has already set the attribute to avoid a flash of
  // the wrong track; this syncs the buttons and wires them up.
  const current = document.documentElement.dataset.audience;
  applyAudience(isAudience(current) ? current : readAudience(), false);

  document.addEventListener('click', (event) => {
    const target = event.target as HTMLElement | null;
    const btn = target?.closest<HTMLElement>('[data-audience-pick]');
    if (!btn) return;
    const next = btn.dataset.audiencePick;
    if (!isAudience(next)) return;
    event.preventDefault();
    document.dispatchEvent(new CustomEvent('bvc:audiencepicked'));
    applyAudience(next);
    scrollToAudience(next);
  });

  startRotation();
}
