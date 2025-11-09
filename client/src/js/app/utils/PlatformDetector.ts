import { platform } from '@tauri-apps/plugin-os';

export default class PlatformDetector {
  private isMobile: boolean | null = null;

  async checkMobile(): Promise<boolean> {
    if (this.isMobile !== null) {
      return this.isMobile;
    }

    try {
      const family = await platform();
      const typeStr = String(family).toLowerCase();
      this.isMobile = typeStr.includes('ios') || typeStr.includes('android');
      return this.isMobile;
    } catch (error) {
      this.isMobile = false;
      return false;
    }
  }

  reset(): void {
    this.isMobile = null;
  }
}
