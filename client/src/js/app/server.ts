import { getVersion } from '@tauri-apps/api/app';
import { info } from '@tauri-apps/plugin-log';
import { writable, type Readable, type Writable } from 'svelte/store';

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

  private appVersionStore: Writable<string>;
  public readonly appVersion: Readable<string>;

  constructor(roster: ServerRosterManager = new ServerRosterManager()) {
    super();
    this.roster = roster;
    this.appVersionStore = writable('');
    this.appVersion = { subscribe: this.appVersionStore.subscribe };
  }

  /**
   * Decide whether this page is the destination.
   *
   * Returns the destination when it is somewhere else, so the caller can keep the boot
   * overlay up. A list that flashed on screen before redirecting past itself would be
   * worse than a slightly longer wait.
   */
  async initialize(): Promise<ServerLanding> {
    if (await this.initializeDeepLinks()) {
      return { kind: 'handoff' };
    }

    void this.loadAppVersion();

    const count = await this.roster.load();

    if (count === 0) {
      info('No servers saved, going to sign-in');
      return { kind: 'navigate', href: ServerRosterManager.SIGN_IN_HREF };
    }

    // A single server is worth waiting for a verdict on, because the verdict may be that
    // this page has nothing to ask. Several are not: the list is the destination either
    // way, so the rows settle in front of the user rather than behind the overlay.
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

  private async loadAppVersion(): Promise<void> {
    this.appVersionStore.set(await getVersion());
  }
}
