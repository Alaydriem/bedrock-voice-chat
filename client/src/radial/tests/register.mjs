import { existsSync } from "node:fs";
import { registerHooks } from "node:module";
import { fileURLToPath } from "node:url";

/**
 * Lets Node's own test runner load the kit's TypeScript directly.
 *
 * Radial adds no test dependency. Node 24 strips types natively, so the only thing
 * missing is extension resolution: the source is written for a bundler and imports
 * `../math/Ease`, while Node's ESM resolver requires `../math/Ease.ts`. This hook
 * appends the extension when the bare specifier does not resolve on its own.
 *
 * Run with:
 *   node --test --experimental-strip-types --import ./src/radial/tests/register.mjs \
 *     "./src/radial/tests/**\/*.test.ts"
 */
registerHooks({
  resolve(specifier, context, nextResolve) {
    const relative = specifier.startsWith("./") || specifier.startsWith("../");
    const hasExtension = /\.[cm]?[jt]sx?$/.test(specifier);
    if (relative && !hasExtension && context.parentURL) {
      for (const extension of [".ts", ".js"]) {
        const candidate = new URL(specifier + extension, context.parentURL);
        if (existsSync(fileURLToPath(candidate))) {
          return nextResolve(specifier + extension, context);
        }
      }
    }
    return nextResolve(specifier, context);
  },
});
