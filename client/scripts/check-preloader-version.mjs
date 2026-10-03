import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

// The build passes whether or not app.html's `%sveltekit.env.PUBLIC_PRELOADER_VERSION%`
// resolved, and an empty `?v=` only shows up as a stale preloader after an update. This
// turns that into a build failure.
const html = readFileSync(fileURLToPath(new URL("../build/index.html", import.meta.url)), "utf8");
const versions = [...html.matchAll(/app-preloader\.(?:js|css)\?v=([^"]*)"/g)].map((m) => m[1]);

if (versions.length !== 2 || versions.some((v) => !/^[0-9a-f]{12}$/.test(v))) {
    process.stderr.write(
        `preloader cache-bust missing from build/index.html: ${JSON.stringify(versions)}\n`,
    );
    process.exit(1);
}
