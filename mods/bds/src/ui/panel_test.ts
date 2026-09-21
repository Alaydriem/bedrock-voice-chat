/// Operator-set count of synthetic rows appended to the volumes view and the
/// Groups page (`/bvc:paneltest <n>`), so their layouts can be exercised at
/// 5/20/50 entries without that many real players or groups. Actions against the
/// synthetic entries are inert — no desktop client matches a synthetic name, and
/// no channel matches a synthetic group id.
export class PanelTestConfig {
  private syntheticCount = 0;

  set(count: number): void {
    this.syntheticCount = Math.max(0, Math.min(100, Math.floor(count)));
  }

  get count(): number {
    return this.syntheticCount;
  }
}
