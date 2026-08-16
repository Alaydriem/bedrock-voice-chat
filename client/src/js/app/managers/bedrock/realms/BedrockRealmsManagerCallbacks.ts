export interface BedrockRealmsManagerCallbacks {
    setStatus: (message: string) => void;
    reportError: (raw: string) => void;
    clearLogs: () => void;
    clearConnectionError: () => void;
    onRealmsUnavailable: () => void;
    /** The stored Xbox credential was rejected and only a fresh sign-in can recover it. */
    onReauthRequired: () => void;
}
