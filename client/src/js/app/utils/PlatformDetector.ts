import { platform } from '@tauri-apps/plugin-os';

export default class PlatformDetector {
  private isMobile: boolean | null = null;

  /**
   * The platform family, without awaiting.
   *
   * `plugin-os` reads a value injected at startup rather than crossing the IPC
   * boundary, so there is nothing to wait for. Awaiting it anyway leaves every caller
   * rendering as desktop for a frame first.
   */
  mobile(): boolean {
    if (this.isMobile !== null) {
      return this.isMobile;
    }

    try {
      const typeStr = String(platform()).toLowerCase();
      this.isMobile = typeStr.includes('ios') || typeStr.includes('android');
    } catch (error) {
      this.isMobile = false;
    }

    return this.isMobile;
  }

  async checkMobile(): Promise<boolean> {
    return this.mobile();
  }

  async isWindows(): Promise<boolean> {
    if (this.isMobile) {
      return false;
    }

    try {
      const family = await platform();
      const typeStr = String(family).toLowerCase();
      if (typeStr.includes("windows")) {
        return true;
      }
    } catch (error) {
    }

    return false;
  }

  reset(): void {
    this.isMobile = null;
  }
}
