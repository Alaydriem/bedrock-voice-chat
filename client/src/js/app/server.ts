import { info } from '@charlesportwoodii/tauri-plugin-curia';

import BVCApp from './BVCApp.ts';

import { AddServerRoute } from './server/AddServerRoute';
import { ServerRosterManager } from './server/ServerRosterManager';
import type { ServerLanding } from './server/ServerLanding';
import type { NextAction } from './shell/NextAction';
import { BootTimeline } from './shell/BootTimeline';

declare global {
  interface Window {
    App: any;
  }
}

export default class Server extends BVCApp {
  public readonly roster: ServerRosterManager;

  constructor(roster: ServerRosterManager = new ServerRosterManager()) {
    super();
    this.roster = roster;
  }

  /**
   * Decide whether this screen is the destination.
   *
   * No saved servers goes to sign-in, and one saved server goes straight to its dashboard once
   * local credentials check out. Returning the destination rather than navigating lets the
   * caller keep the boot overlay up over both, so the list cannot flash on its way past.
   *
   * The preflight never gates this. It is the list's diagnostic — it explains a server that
   * will not work to somebody choosing between several — and the connect path measures the
   * network for itself either way.
   */
  async initialize(): Promise<ServerLanding> {
    const timeline = BootTimeline.shared();

    if (await this.initializeDeepLinks()) {
      return { kind: 'handoff' };
    }
    timeline.mark('/ deep links');

    const count = await this.roster.load();
    timeline.mark(`/ roster.load (${count} server(s))`);

    if (count === 0) {
      info('No servers saved, going to sign-in');
      return { kind: 'navigate', href: ServerRosterManager.SIGN_IN_HREF };
    }

    if (count === 1) {
      const sole = await this.roster.soleDestination();
      timeline.mark('/ soleDestination (credentials + cert, local)');
      if (sole.kind === 'navigate') return sole;
    }

    // The list is the destination, so the plates settle in front of the user rather than
    // behind the overlay.
    void this.roster.sweep();
    return { kind: 'show' };
  }

  addServer(): NextAction {
    return { kind: 'navigate', href: AddServerRoute.HREF };
  }

  showPreloader(): void {
    this.preloader();
  }
}
