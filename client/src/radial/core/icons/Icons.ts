/**
 * The icon set.
 *
 * Stroke-built inline SVG: no icon font, no CDN, no network at all. The client ships
 * offline and runs inside a WebView, so an icon that needs a request is an icon that
 * sometimes is not there.
 *
 * Everything shares one 24x24 box, a 1.9 stroke and round caps, which is what keeps
 * a mic beside a headphone beside a gear reading as one family. Where a shape needs
 * a filled centre — the record dot, a kebab, a grip — it says so locally.
 *
 * The prototype carried two icon maps with two different `gear` drawings at
 * different weights. This is the stroked one; the filled variant is gone.
 */
export const RAD_ICONS = {
  mic: '<rect x="9" y="2.5" width="6" height="11" rx="3"/><path d="M5.5 11.5a6.5 6.5 0 0 0 13 0"/><path d="M12 18v3.5"/>',
  micoff:
    '<rect x="9" y="2.5" width="6" height="11" rx="3"/><path d="M5.5 11.5a6.5 6.5 0 0 0 13 0"/><path d="M12 18v3.5"/><path d="M4 3l16 18"/>',
  head: '<path d="M4 15.5v-3.5a8 8 0 0 1 16 0v3.5"/><rect x="2.5" y="14" width="4.6" height="7.2" rx="2.3"/><rect x="16.9" y="14" width="4.6" height="7.2" rx="2.3"/>',
  headoff:
    '<path d="M4 15.5v-3.5a8 8 0 0 1 16 0v3.5"/><rect x="2.5" y="14" width="4.6" height="7.2" rx="2.3"/><rect x="16.9" y="14" width="4.6" height="7.2" rx="2.3"/><path d="M3 2.5l18 19"/>',
  speakeroff:
    '<path d="M4 9.4h3.1L12.7 5v14l-5.6-4.4H4z"/><path d="M16.4 9.6a4.6 4.6 0 0 1 .6 4.4"/><path d="M19.4 6.6a8.4 8.4 0 0 1 .9 8.6"/><path d="M3.4 3.4l17.2 17.2"/>',
  /**
   * A waveform, for a device whose format the app cannot use — a sample rate or a
   * channel count, not a device that is missing. `micoff` says "there is nothing
   * there", which is the opposite problem.
   */
  wave: '<path d="M2.6 12h2.6l2-6.4 3 12.8 3-9.6 1.8 3.2h6.4"/>',
  belloff:
    '<path d="M6.6 10.6a5.4 5.4 0 0 1 9.2-3.9"/><path d="M17.4 12.6v-2a5.4 5.4 0 0 0-.2-1.5"/><path d="M5.4 16.4s1.2-1.3 1.2-5.8"/><path d="M17.4 12.6c0 3.2 1.2 3.8 1.2 3.8H5.4"/><path d="M10 19.3a2.3 2.3 0 0 0 4 0"/><path d="M3.6 3.4l16.8 17.2"/>',
  rec: '<circle cx="12" cy="12" r="8.2"/><circle cx="12" cy="12" r="3.6" fill="currentColor" stroke="none"/>',
  refresh: '<path d="M20.5 12a8.5 8.5 0 1 1-2.6-6.1"/><path d="M20.5 3.5V9h-5.5"/>',
  plus: '<path d="M12 4.5v15"/><path d="M4.5 12h15"/>',
  minus: '<path d="M4.5 12h15"/>',
  close: '<path d="M6.5 6.5l11 11"/><path d="M17.5 6.5l-11 11"/>',
  chat: '<path d="M20.5 12.2c0 4.1-3.8 7.4-8.5 7.4a9.8 9.8 0 0 1-2.6-.35L4.2 20.8l1.3-3.7A7 7 0 0 1 3.5 12.2C3.5 8.1 7.3 4.8 12 4.8s8.5 3.3 8.5 7.4Z"/>',
  send: '<path d="M4 11.6 20.2 4.4 13.6 20.6l-2.3-6.6z"/><path d="M11.3 14 20.2 4.4"/>',
  /** The proximity field: what the dashboard's status view is a reading of. */
  field:
    '<circle cx="12" cy="12" r="9"/><circle cx="12" cy="12" r="4.6"/><circle cx="12" cy="12" r="1.3" fill="currentColor" stroke="none"/>',
  people:
    '<circle cx="9" cy="8" r="3.4"/><path d="M2.8 20.2a6.2 6.2 0 0 1 12.4 0"/><path d="M16.2 5.2a3.4 3.4 0 0 1 0 6.8"/><path d="M17.6 14.6a6.2 6.2 0 0 1 3.6 5.6"/>',
  /**
   * The filled cog, not the eight-spoke stroked one. A ring of spokes around a small
   * circle reads as a sun at this size, which is not what a settings button should
   * look like. This is the shape the design system's dashboard used.
   */
  gear:
    '<g stroke="none" fill="currentColor">' +
    '<path fill-opacity=".32" d="M2 12.947v-1.771c0-1.047.85-1.913 1.899-1.913 1.81 0 2.549-1.288 1.64-2.868a1.919 1.919 0 0 1 .699-2.607l1.729-.996c.79-.474 1.81-.192 2.279.603l.11.192c.9 1.58 2.379 1.58 3.288 0l.11-.192c.47-.795 1.49-1.077 2.279-.603l1.73.996a1.92 1.92 0 0 1 .699 2.607c-.91 1.58-.17 2.868 1.639 2.868 1.04 0 1.899.856 1.899 1.912v1.772c0 1.047-.85 1.912-1.9 1.912-1.808 0-2.548 1.288-1.638 2.869.52.915.21 2.083-.7 2.606l-1.729.997c-.79.473-1.81.191-2.279-.604l-.11-.191c-.9-1.58-2.379-1.58-3.288 0l-.11.19c-.47.796-1.49 1.078-2.279.605l-1.73-.997a1.919 1.919 0 0 1-.699-2.606c.91-1.58.17-2.869-1.639-2.869A1.911 1.911 0 0 1 2 12.947Z"/>' +
    '<path d="M11.995 15.332c1.794 0 3.248-1.464 3.248-3.27 0-1.807-1.454-3.272-3.248-3.272-1.794 0-3.248 1.465-3.248 3.271 0 1.807 1.454 3.271 3.248 3.271Z"/>' +
    "</g>",
  kebab:
    '<circle cx="12" cy="5" r="1.6" fill="currentColor" stroke="none"/><circle cx="12" cy="12" r="1.6" fill="currentColor" stroke="none"/><circle cx="12" cy="19" r="1.6" fill="currentColor" stroke="none"/>',
  chev: '<path d="M6 9.5l6 6 6-6"/>',
  back: '<path d="M15 5l-7 7 7 7"/>',
  copy: '<rect x="9" y="9" width="11.5" height="11.5" rx="2.4"/><path d="M15 5.5H6a2.5 2.5 0 0 0-2.5 2.5v9"/>',
  check: '<path d="M5 12.8l4.6 4.6L19 7"/>',
  ext: '<path d="M14 4.5h5.5V10"/><path d="M19.5 4.5L11 13"/><path d="M18 14.5v4a2 2 0 0 1-2 2H5.5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h4"/>',
  play: '<path d="M7 4.8v14.4L19.5 12z"/>',
  stop: '<rect x="6" y="6" width="12" height="12" rx="2"/>',
  trash:
    '<path d="M4.5 6.5h15"/><path d="M9.5 6.5V4.8h5v1.7"/><path d="M6.6 6.5l.9 12.2a2 2 0 0 0 2 1.8h5a2 2 0 0 0 2-1.8l.9-12.2"/>',
  search: '<circle cx="10.8" cy="10.8" r="6.6"/><path d="M15.6 15.6L20.5 20.5"/>',
  terminal:
    '<rect x="2.8" y="4.3" width="18.4" height="15.4" rx="2.4"/><path d="M7 9.5l3 2.6-3 2.6"/><path d="M12.4 15h4.4"/>',
  grip: '<circle cx="9" cy="6" r="1.5" fill="currentColor" stroke="none"/><circle cx="15" cy="6" r="1.5" fill="currentColor" stroke="none"/><circle cx="9" cy="12" r="1.5" fill="currentColor" stroke="none"/><circle cx="15" cy="12" r="1.5" fill="currentColor" stroke="none"/><circle cx="9" cy="18" r="1.5" fill="currentColor" stroke="none"/><circle cx="15" cy="18" r="1.5" fill="currentColor" stroke="none"/>',
  globe:
    '<circle cx="12" cy="12" r="8.8"/><path d="M3.5 9.4h17"/><path d="M3.5 14.6h17"/><path d="M12 3.2c2.4 2.4 3.6 5.3 3.6 8.8S14.4 18.4 12 20.8c-2.4-2.4-3.6-5.3-3.6-8.8S9.6 5.6 12 3.2Z"/>',
  server:
    '<rect x="3.4" y="3.8" width="17.2" height="6.8" rx="2.1"/><rect x="3.4" y="13.4" width="17.2" height="6.8" rx="2.1"/><path d="M7.4 7.2h.02"/><path d="M7.4 16.8h.02"/><path d="M11.4 7.2h4.8"/><path d="M11.4 16.8h4.8"/>',
  /** A chain pulled apart, for a path that was there and is not. */
  unlink:
    '<path d="M9.6 14.4 7.2 16.8a3.7 3.7 0 0 1-5.2-5.2l2.4-2.4"/><path d="M14.4 9.6l2.4-2.4a3.7 3.7 0 0 1 5.2 5.2l-2.4 2.4"/><path d="M12.6 3.4v2.4"/><path d="M3.4 12.6h2.4"/><path d="M18.2 11.4h2.4"/><path d="M11.4 18.2v2.4"/>',
  /**
   * A tick inside, not a cross. This is the parental gate: the shield is the protection
   * working as intended, and a child looking at it has not done anything wrong.
   */
  shield:
    '<path d="M12 3.2 20 6v5.4c0 4.6-3.2 7.7-8 9.4-4.8-1.7-8-4.8-8-9.4V6z"/><path d="M9.2 12.1l2.1 2.2 3.9-4.2"/>',
  shieldoff:
    '<path d="M12 3.2 20 6v5.4c0 4.6-3.2 7.7-8 9.4-4.8-1.7-8-4.8-8-9.4V6z"/><path d="M8.4 8.4l7.2 7.2"/><path d="M15.6 8.4l-7.2 7.2"/>',
  cert:
    '<rect x="3.4" y="4.2" width="17.2" height="11.4" rx="2.2"/><path d="M7.4 8.4h5.2"/><path d="M7.4 11.6h3"/><circle cx="16.6" cy="10.2" r="2.2"/><path d="M14.9 12l-.7 3.6 2.4-1.4 2.4 1.4-.7-3.6"/>',
  lock:
    '<rect x="4.6" y="10.2" width="14.8" height="10.2" rx="2.4"/><path d="M8.2 10.2V7.6a3.8 3.8 0 0 1 7.6 0v2.6"/><path d="M12 14.2v2.4"/>',
  download:
    '<path d="M12 3.6v10.8"/><path d="M7.6 10.2 12 14.6l4.4-4.4"/><path d="M4.4 19.4h15.2"/>',
  warn: '<path d="M12 3.5 21.5 20H2.5z"/><path d="M12 9.5v5"/><circle cx="12" cy="17.4" r="1" fill="currentColor" stroke="none"/>',
  info: '<circle cx="12" cy="12" r="9"/><path d="M12 11v6"/><circle cx="12" cy="7.6" r="1" fill="currentColor" stroke="none"/>',
} as const;

export type IconName = keyof typeof RAD_ICONS;

export class Icons {
  static has(name: string): name is IconName {
    return name in RAD_ICONS;
  }

  static names(): IconName[] {
    return Object.keys(RAD_ICONS) as IconName[];
  }

  /** Markup for one icon. Decorative by default; label the button, not the glyph. */
  static svg(name: IconName): string {
    return (
      '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" ' +
      'stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">' +
      RAD_ICONS[name] +
      "</svg>"
    );
  }
}
