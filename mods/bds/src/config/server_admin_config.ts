type ServerAdminModule = typeof import('@minecraft/server-admin');

export class ServerAdminConfig {
  private loadPromise: Promise<void> | null = null;
  private loaded = false;
  private module: ServerAdminModule | null = null;
  private warned = false;

  private serverUrl = '';
  private token = '';
  private minPlayers = 1;

  get bvcServer(): string {
    return this.serverUrl;
  }

  get accessToken(): string {
    return this.token;
  }

  get minimumPlayers(): number {
    return this.minPlayers;
  }

  isAvailable(): boolean {
    return this.loaded && this.module !== null;
  }

  async ensureLoaded(): Promise<boolean> {
    if (this.loaded) {
      return this.module !== null;
    }
    if (!this.loadPromise) {
      this.loadPromise = import('@minecraft/server-admin')
        .then((m) => {
          this.module = m as unknown as ServerAdminModule;
          const variables = m.variables;

          const server = variables.get('bvc_server');
          if (typeof server === 'string') {
            this.serverUrl = server;
          }

          const accessToken = variables.get('bvc_access_token');
          if (typeof accessToken === 'string') {
            this.token = accessToken;
          }

          const minimum = variables.get('bvc_minimum_players');
          if (typeof minimum === 'number' && minimum >= 1) {
            this.minPlayers = Math.floor(minimum);
          }
        })
        .catch((e) => {
          this.module = null;
          if (!this.warned) {
            console.warn(
              '[BVC] @minecraft/server-admin not available; server variables disabled: ' +
                e,
            );
            this.warned = true;
          }
        })
        .then(() => {
          this.loaded = true;
        });
    }
    await this.loadPromise;
    return this.module !== null;
  }
}

export const serverAdminConfig = new ServerAdminConfig();
