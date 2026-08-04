/**
 * A button on a fault screen.
 *
 * `url` is either a route or an external address, and the screen decides which by the
 * scheme. An empty string is a fault whose primary button does something instead of going
 * somewhere — the update installs itself.
 */
export default interface FaultAction {
  label: string;
  url: string;
}
