import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, resolve, relative, sep } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const bdsDir = resolve(here, '..');

const eslintBin = resolve(bdsDir, 'node_modules', 'eslint', 'bin', 'eslint.js');
const prettierBin = resolve(
  bdsDir,
  'node_modules',
  'prettier',
  'bin',
  'prettier.cjs',
);

async function readStdin() {
  let raw = '';
  process.stdin.setEncoding('utf8');
  for await (const chunk of process.stdin) {
    raw += chunk;
  }
  return raw;
}

function isBdsSourceFile(filePath) {
  const abs = resolve(filePath);
  const rel = relative(bdsDir, abs).split(sep).join('/');
  return !rel.startsWith('../') && rel.startsWith('src/') && abs.endsWith('.ts');
}

function run(bin, file) {
  try {
    execFileSync(process.execPath, [bin, '--fix', file], {
      cwd: bdsDir,
      stdio: 'ignore',
    });
  } catch {
    // Never block the edit on a tooling failure.
  }
}

const raw = await readStdin();

let filePath;
try {
  filePath = JSON.parse(raw || '{}')?.tool_input?.file_path;
} catch {
  process.exit(0);
}

if (!filePath || !isBdsSourceFile(filePath)) {
  process.exit(0);
}

const abs = resolve(filePath);
run(eslintBin, abs);
try {
  execFileSync(process.execPath, [prettierBin, '--write', abs], {
    cwd: bdsDir,
    stdio: 'ignore',
  });
} catch {
  // Never block the edit on a tooling failure.
}
process.exit(0);
