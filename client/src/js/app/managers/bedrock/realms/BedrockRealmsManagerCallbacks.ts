export interface BedrockRealmsManagerCallbacks {
    setStatus: (message: string) => void;
    reportError: (raw: string) => void;
    clearLogs: () => void;
    clearConnectionError: () => void;
    onRealmsUnavailable: () => void;
}
