import { Store } from '@tauri-apps/plugin-store';

/**
 * The one handle to `store.json`.
 *
 * Every `Store.load` is a `path/getDataDir` round trip that Tauri runs on Android's UI thread,
 * and a launch used to make several of them during the window when that thread is least
 * available — tearing down the splash, compiling GPU pipelines, laying out the webview for the
 * first time. A call that lands in there waits for the thread rather than for any work of its
 * own, which is why the cost moved between different calls from one launch to the next.
 *
 * Sharing the promise rather than the store, so concurrent callers during startup queue on one
 * in-flight load instead of racing to start several.
 */
export class AppStore {
    private static readonly PATH = 'store.json';

    private static handle: Promise<Store> | null = null;

    static load(): Promise<Store> {
        AppStore.handle ??= Store.load(AppStore.PATH, { autoSave: false, defaults: {} });
        return AppStore.handle;
    }
}
