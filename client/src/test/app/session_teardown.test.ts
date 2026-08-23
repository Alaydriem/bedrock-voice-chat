import { beforeEach, describe, expect, it, vi } from "vitest";
import { mockInvoke } from "../tauri";

const { default: BVCApp } = await import("../../js/app/BVCApp");

/**
 * What a client-side back navigation may and may not tear down.
 *
 * `BVCApp` registers its teardown on the events that mean the document is going away, and
 * `popstate` was among them. It is not one: it fires on a move *within* the document, and
 * settings is a child route that is left by popping history — so closing the settings cover ran
 * the entire session teardown while the dashboard was still on screen. Every level consumer was
 * unsubscribed, the self controller and both managers were dropped, and because the layout was
 * never destroyed nothing re-initialised them. The pill and every player card stayed flat for
 * the life of the page while the link was up and levels were still arriving, which is why this
 * read as a meter fault for a long time and survived a rewrite of the transport underneath it.
 */
class SpyApp extends BVCApp {
    cleanups = 0;

    async cleanup(): Promise<void> {
        this.cleanups += 1;
    }
}

beforeEach(() => {
    mockInvoke({});
});

describe("session teardown", () => {
    it("survives a client-side back navigation", async () => {
        const app = new SpyApp();

        window.dispatchEvent(new PopStateEvent("popstate"));
        await Promise.resolve();

        expect(app.cleanups).toBe(0);
    });

    // The document really is going away here, and the listeners this instance registered are
    // process-wide: nothing else releases them.
    it.each(["pagehide", "beforeunload", "unload"])("still tears down on %s", async (name) => {
        const app = new SpyApp();

        window.dispatchEvent(new Event(name));
        await Promise.resolve();

        expect(app.cleanups).toBe(1);
    });
});
