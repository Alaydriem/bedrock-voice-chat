export interface BedrockAuthManagerCallbacks {
    setStatus: (message: string) => void;
    onLoginSuccess: () => Promise<void>;
}
