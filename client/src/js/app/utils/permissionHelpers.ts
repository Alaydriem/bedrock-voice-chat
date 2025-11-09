import { checkPermission, requestPermission, PermissionType, type PermissionResponse } from 'tauri-plugin-audio-permissions';
import { error as logError } from '@tauri-apps/plugin-log';

/**
 * Wraps a promise with a timeout
 * @param promise The promise to wrap
 * @param ms Timeout in milliseconds
 * @returns Promise that rejects if timeout is reached
 */
function withTimeout<T>(promise: Promise<T>, ms: number): Promise<T> {
    return Promise.race([
        promise,
        new Promise<T>((_, reject) =>
            setTimeout(() => reject(new Error('Permission request timeout')), ms)
        )
    ]);
}

/**
 * Checks current permission status
 * @param permissionType The type of permission to check
 * @returns Promise with permission response
 */
export async function checkPermissionStatus(
    permissionType: PermissionType
): Promise<PermissionResponse> {
    try {
        const response = await checkPermission({ permissionType });
        return response;
    } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        await logError(`Permission check failed: ${errorMessage}`);
        throw error;
    }
}

/**
 * Requests permission with timeout and error handling
 * @param permissionType The type of permission to request
 * @param timeoutMs Timeout in milliseconds (default: 10000)
 * @returns Promise with permission response
 */
export async function requestPermissionWithTimeout(
    permissionType: PermissionType,
    timeoutMs: number = 10000
): Promise<PermissionResponse> {
    try {
        const response = await withTimeout(
            requestPermission({ permissionType }),
            timeoutMs
        );
        return response;
    } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        await logError(`Permission request failed: ${errorMessage}`);
        throw error;
    }
}
