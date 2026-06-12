import type { RealmsGateStatus } from '../../../../bindings/RealmsGateStatus';

export interface BedrockRealmsManagerCallbacks {
    setStatus: (message: string) => void;
    reportError: (raw: string) => void;
    clearLogs: () => void;
    clearConnectionError: () => void;
    onGateBlocked: (status: RealmsGateStatus) => void;
}
