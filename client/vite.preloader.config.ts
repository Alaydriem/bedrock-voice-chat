import { defineConfig } from "vite";
import { fileURLToPath } from "node:url";

/**
 * Builds the boot preloader into `static/preloader/app-preloader.js`.
 *
 * A separate config because this is not part of the app bundle and must not be: the
 * overlay's job is to be on screen when the bundle is slow or broken, so it ships as
 * a standalone IIFE with everything compiled in and no runtime imports.
 *
 * Output is committed. Regenerate with `yarn preloader:build`, which `dev` and `build`
 * both run first so it cannot silently fall behind the kit it draws from.
 */
export default defineConfig({
    resolve: {
        alias: {
            $radial: fileURLToPath(new URL("./src/radial", import.meta.url)),
        },
    },
    build: {
        outDir: "static/preloader",
        // The hand-written CSS lives here too and is not generated.
        emptyOutDir: false,
        target: ["es2020", "chrome87"],
        lib: {
            entry: fileURLToPath(new URL("./src/preloader/index.ts", import.meta.url)),
            formats: ["iife"],
            name: "BvcPreloader",
            fileName: () => "app-preloader.js",
        },
    },
});
