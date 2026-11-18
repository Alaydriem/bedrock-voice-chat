import type { HostCapabilityReport } from '../dto/host_capability_report';

const HOST_CAPABILITY_PATH = '/api/telemetry/host-capability';

export class HostCapabilityReporter {
  private sent = false;

  constructor(
    private readonly post: (path: string, body: string) => Promise<void>,
    private readonly telemetry: () => boolean,
    private readonly version: string,
  ) {}

  async report(): Promise<void> {
    if (this.sent || !this.telemetry()) {
      return;
    }
    this.sent = true;

    const report: HostCapabilityReport = {
      mod_version: this.version,
      loader: 'bds',
      mode: 'external',
    };
    try {
      await this.post(HOST_CAPABILITY_PATH, JSON.stringify(report));
    } catch (error) {
      console.warn('[BVC] Host capability report failed: ' + error);
    }
  }
}
