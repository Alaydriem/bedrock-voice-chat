import type { BedrockConnectError } from '../../../../bindings/BedrockConnectError';
import type { RealmsConnectionError } from './RealmsConnectionError';

type OtherConnectError = Extract<BedrockConnectError, { kind: 'other' }>;

export class BedrockConnectErrorMapper {
    static describe(err: BedrockConnectError): RealmsConnectionError {
        switch (err.kind) {
            case 'nethernet_rejected_no_fallback': {
                const codeText = err.bds_reason_code != null ? `reason ${err.bds_reason_code}` : 'no reason code';
                const kick = err.bds_kick_message && err.bds_kick_message.length > 0
                    ? ` "${err.bds_kick_message}"`
                    : '';
                return {
                    raw: err,
                    title: 'Bedrock server rejected the login',
                    detail: `The Realm rejected the proxy handshake (${codeText}${kick}) and there is no fallback for NetherNet/Realms.`,
                    suggestion: 'This usually clears in 30 seconds. Wait, then try Connect again. If it persists, click Refresh to renew tokens, or restart BVC.',
                    severity: 'error',
                };
            }
            case 'bds_rejected_original_login': {
                const kick = err.kick_message && err.kick_message.length > 0
                    ? ` ("${err.kick_message}")`
                    : '';
                return {
                    raw: err,
                    title: 'Server rejected the original login',
                    detail: `Even the unmodified login chain was rejected${kick}.`,
                    suggestion: 'Click Refresh to renew tokens, then try again. If it persists, sign out and back in.',
                    severity: 'error',
                };
            }
            case 'bds_rejected_original_login_undecoded':
                return {
                    raw: err,
                    title: 'Server rejected the original login',
                    detail: 'The server sent a Disconnect packet during fallback that could not be decoded.',
                    suggestion: 'Click Refresh to renew tokens, then try again.',
                    severity: 'error',
                };
            case 'handshake_other':
                return {
                    raw: err,
                    title: 'Handshake failed',
                    detail: err.message,
                    suggestion: 'Wait a moment and try Connect again. If it keeps failing, click Refresh or restart BVC.',
                    severity: 'error',
                };
            case 'auth':
                return {
                    raw: err,
                    title: 'Authentication failed',
                    detail: err.message,
                    suggestion: 'Click Refresh to renew tokens. If it persists, sign out and back in.',
                    severity: 'error',
                };
            case 'reauth_required':
                return {
                    raw: err,
                    title: 'Sign in to Xbox again',
                    detail: 'Your Xbox Live session expired and could not be renewed.',
                    suggestion: 'Sign in with the code shown to get back to your worlds.',
                    severity: 'error',
                };
            case 'transport':
                return {
                    raw: err,
                    title: 'Network transport failed',
                    detail: err.message,
                    suggestion: 'Check your internet connection. If it persists, restart BVC.',
                    severity: 'error',
                };
            case 'other':
            default:
                return {
                    raw: err,
                    title: 'Connect failed',
                    detail: (err as OtherConnectError).message ?? 'Unknown error',
                    suggestion: 'Wait a moment and try Connect again.',
                    severity: 'error',
                };
        }
    }
}
