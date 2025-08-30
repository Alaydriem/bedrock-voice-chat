import { family } from '@tauri-apps/plugin-os';

class PlatformDetector {
  constructor() {
    this.isMobile = null;
  }

  async checkMobile() {
    if (this.isMobile !== null) {
      return this.isMobile;
    }

    try {
      const osFamily = await family();
      const familyStr = String(osFamily).toLowerCase();
      this.isMobile = familyStr.includes('ios') || familyStr.includes('android');
      return this.isMobile;
    } catch (error) {
      console.warn('Could not detect OS family:', error);
      this.isMobile = false;
      return false;
    }
  }

  reset() {
    this.isMobile = null;
  }
}

export const platformDetector = new PlatformDetector();
