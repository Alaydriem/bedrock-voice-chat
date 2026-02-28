// Polyfills for Android 11 WebView (Chrome ~87)

if (!String.prototype.replaceAll) {
    String.prototype.replaceAll = function (
        search: string | RegExp,
        replacement: string | ((substring: string, ...args: unknown[]) => string),
    ): string {
        if (search instanceof RegExp) {
            if (!search.global) {
                throw new TypeError("replaceAll must be called with a global RegExp");
            }
            return this.replace(search, replacement as string);
        }
        return this.split(search).join(replacement as string);
    };
}

if (typeof globalThis.queueMicrotask !== "function") {
    globalThis.queueMicrotask = (cb: VoidFunction) => {
        Promise.resolve().then(cb);
    };
}

if (typeof globalThis.structuredClone !== "function") {
    globalThis.structuredClone = <T>(value: T): T => {
        return JSON.parse(JSON.stringify(value));
    };
}

if (!Array.prototype.at) {
    Array.prototype.at = function <T>(this: T[], index: number): T | undefined {
        const i = index >= 0 ? index : this.length + index;
        return this[i];
    };
}
