import { family } from '@tauri-apps/plugin-os';

export default class PlatformDetector {
  private isMobile: boolean | null = null;

  async checkMobile(): Promise<boolean> {
    if (this.isMobile !== null) {
      return this.isMobile;
    }

    try {
      const osFamily = await family();
      const familyStr = String(osFamily).toLowerCase();
      this.isMobile = familyStr.includes('ios') || familyStr.includes('android');
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
