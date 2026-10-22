// Runs the add-on's unit tests: bundles each tests/**/*.test.ts with esbuild into .test-out/
// and runs them with Node's own test runner. Only modules that import nothing from
// @minecraft/* can be tested this way; keep the logic under test in such modules.
const esbuild = require('esbuild');
const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const root = __dirname;
const testsDir = path.join(root, 'tests');
const outDir = path.join(root, '.test-out');

function findTests(dir) {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      return findTests(full);
    }
    return entry.name.endsWith('.test.ts') ? [full] : [];
  });
}

async function run() {
  fs.rmSync(outDir, { recursive: true, force: true });
  const tests = findTests(testsDir);
  if (tests.length === 0) {
    console.error('no tests found under tests/');
    process.exit(1);
  }
  await esbuild.build({
    entryPoints: tests,
    bundle: true,
    outdir: outDir,
    outbase: testsDir,
    outExtension: { '.js': '.mjs' },
    platform: 'node',
    format: 'esm',
    target: 'node18',
    logLevel: 'warning',
  });
  const outputs = findTests(testsDir).map((file) =>
    path.join(outDir, path.relative(testsDir, file).replace(/\.ts$/, '.mjs')),
  );
  const result = spawnSync(process.execPath, ['--test', ...outputs], { stdio: 'inherit' });
  process.exit(result.status ?? 1);
}

run().catch((error) => {
  console.error(error);
  process.exit(1);
});
