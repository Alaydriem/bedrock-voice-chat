export interface RealmsLifecycle {
    isRunning(): boolean;
    stopRealms(): Promise<void>;
}
