/// Operator-set count of synthetic rows appended to the volumes view
/// (`/bvc:paneltest <n>`), so its layout can be exercised at 5/20/50 players
/// without that many real bodies nearby. Actions against the synthetic names
/// are inert — no desktop client matches them.
export class PanelTestConfig {
  private syntheticCount = 0;

  set(count: number): void {
    this.syntheticCount = Math.max(0, Math.min(100, Math.floor(count)));
  }

  get count(): number {
    return this.syntheticCount;
  }
}
