/**
 * Mobile nav. The old implementation kept three module-level booleans in sync
 * by hand and leaked a window click listener on every toggle; this reads state
 * off the DOM so there is only one copy of it.
 */
export function initNav(): void {
  const toggle = document.querySelector<HTMLButtonElement>('[data-nav-toggle]');
  const panel = document.querySelector<HTMLElement>('[data-nav-panel]');
  if (!toggle || !panel) return;

  const isOpen = (): boolean => toggle.getAttribute('aria-expanded') === 'true';

  const setOpen = (open: boolean): void => {
    toggle.setAttribute('aria-expanded', String(open));
    panel.classList.toggle('is-open', open);
  };

  toggle.addEventListener('click', (event) => {
    event.stopPropagation();
    setOpen(!isOpen());
  });

  document.addEventListener('click', (event) => {
    if (!isOpen()) return;
    if (panel.contains(event.target as Node)) return;
    setOpen(false);
  });

  document.addEventListener('keydown', (event) => {
    if (event.key === 'Escape' && isOpen()) {
      setOpen(false);
      toggle.focus();
    }
  });

  // Choosing a destination should close the menu.
  panel.addEventListener('click', (event) => {
    if ((event.target as HTMLElement).closest('a')) setOpen(false);
  });
}
