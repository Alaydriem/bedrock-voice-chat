import type { ServerHealthStatus } from './ServerHealthStatus';

export interface ServerHealthResult {
    status: ServerHealthStatus;
    compatible: boolean;
    clientTooOld: boolean;
    serverVersion: string;
    clientVersion: string;
}
