import type { BedrockConnectError } from '../../../../bindings/BedrockConnectError';

export interface RealmsConnectionError {
    raw: BedrockConnectError;
    title: string;
    detail: string;
    suggestion: string;
    severity: 'error' | 'warning';
}
