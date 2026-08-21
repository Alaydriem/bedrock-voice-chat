import { invoke } from "@tauri-apps/api/core";
import { debug, error, info, trace, warn } from "@charlesportwoodii/tauri-plugin-curia";

/**
 * Fires the webview half of the logging smoke test, then asks the Rust half to
 * run. Every line carries `smoke_test: true` so synthetic data can be filtered
 * out of a real Sentry view.
 *
 * The webview half is the part with no other coverage: it is the only thing that
 * exercises the `curia:default` capability grant and the JSON field boundary.
 */
export class LoggingSmokeTest {
    static async run(): Promise<void> {
        await LoggingSmokeTest.levels();
        await LoggingSmokeTest.nestedFields();
        await LoggingSmokeTest.defectRouting();

        // The Rust half last, so a failure above is not masked by it
        await invoke("logging_smoke_test");
    }

    private static async levels(): Promise<void> {
        await error("smoke ts: error level", { smoke_test: true });
        await warn("smoke ts: warn level", { smoke_test: true });
        await info("smoke ts: info level", { smoke_test: true });
        await debug("smoke ts: debug level", { smoke_test: true });
        await trace("smoke ts: trace level", { smoke_test: true });
    }

    /** A number must arrive numeric, and nesting must survive intact. */
    private static async nestedFields(): Promise<void> {
        await warn("smoke ts: nested fields", {
            smoke_test: true,
            frames: 1024,
            live: true,
            detail: { ok: true, codes: [1, 2, 3] },
        });
    }

    private static async defectRouting(): Promise<void> {
        // Expect one Issue, grouped with the Rust-side AudioDeviceLost
        await error("smoke ts: declared defect", {
            smoke_test: true,
            defect: "AudioDeviceLost",
            error: "synthetic",
        });

        // Unknown value: expect demotion to an attribute, and no Issue
        await error("smoke ts: unknown defect", {
            smoke_test: true,
            defect: "NotARealDefect",
            error: "synthetic",
        });
    }
}
