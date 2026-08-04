import { info } from '@tauri-apps/plugin-log';

import BVCApp from './BVCApp.ts';

import { ServerRosterManager } from './server/ServerRosterManager';
import type { ServerLanding } from './server/ServerLanding';
import type { NextAction } from './shell/NextAction';

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
   * Decide whether this page is the destination.
   *
   * Two of the four cases never reach this screen: no saved servers goes to sign-in, and one
   * saved server that passes its preflight goes straight to the dashboard. Returning the
   * destination rather than navigating lets the caller keep the boot overlay up over both, so
   * the list cannot flash on its way past.
   */
  async initialize(): Promise<ServerLanding> {
    if (await this.initializeDeepLinks()) {
      return { kind: 'handoff' };
    }

    const count = await this.roster.load();

    if (count === 0) {
      info('No servers saved, going to sign-in');
      return { kind: 'navigate', href: ServerRosterManager.SIGN_IN_HREF };
    }

    // A single server is worth waiting for a verdict on, because the verdict may be that this
    // page has nothing to ask. Several are not: the list is the destination either way, so
    // the plates settle in front of the user rather than behind the overlay.
    if (count === 1) {
      await this.roster.sweep();
      const only = ServerRosterManager.autoJoin(this.roster.current());
      if (only) {
        const next = await this.roster.choose(only);
        if (next.kind === 'navigate') return next;
      }
    } else {
      void this.roster.sweep();
    }

    return { kind: 'show' };
  }

  addServer(): NextAction {
    return { kind: 'navigate', href: ServerRosterManager.ADD_HREF };
  }

  showPreloader(): void {
    this.preloader();
  }
}
