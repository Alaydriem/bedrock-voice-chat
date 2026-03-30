const { execSync } = require('child_process');
const { existsSync, mkdirSync, copyFileSync } = require('fs');
const path = require('path');

const args = process.argv.slice(2);
const buildAll = args.includes('--all');
const release = args.includes('--release');

function getArg(name) {
  const arg = args.find(a => a.startsWith(`--${name}=`));
  return arg ? arg.split('=').slice(1).join('=') : null;
}

const fabricDest = getArg('fabric-dest');
const paperDest = getArg('paper-dest');
const hytaleDest = getArg('hytale-dest');
const bdsBpDest = getArg('bds-bp-dest');
const bdsRpDest = getArg('bds-rp-dest');

const modsDir = __dirname;
const javaDir = path.join(modsDir, 'java');
const bdsDir = path.join(modsDir, 'bds');

const isWindows = process.platform === 'win32';
const gradlew = isWindows ? '.\\gradlew.bat' : './gradlew';

// Step 1: Build Java mods (Rust lib + Paper + Hytale, optionally Fabric)
const gradleTask = buildAll ? 'devBuildAll' : 'devBuild';
const gradleArgs = [];
if (fabricDest) gradleArgs.push(`-PfabricDest=${fabricDest}`);
if (paperDest) gradleArgs.push(`-PpaperDest=${paperDest}`);
if (hytaleDest) gradleArgs.push(`-PhytaleDest=${hytaleDest}`);
if (release) gradleArgs.push('-Prelease');

const gradleCmd = [gradlew, gradleTask, ...gradleArgs].join(' ');
console.log(`\n=== Java Mods (${gradleTask}) ===\n`);
execSync(gradleCmd, { cwd: javaDir, stdio: 'inherit' });

// Step 2: Build BDS pack (only with --all)
if (buildAll) {
  console.log('\n=== BDS Pack ===\n');
  execSync('yarn run pack', { cwd: bdsDir, stdio: 'inherit' });

  const bpMcpack = path.join(bdsDir, 'bedrock-voice-chat-bp.mcpack');
  const rpMcpack = path.join(bdsDir, 'bedrock-voice-chat-rp.mcpack');

  if (bdsBpDest && existsSync(bpMcpack)) {
    mkdirSync(bdsBpDest, { recursive: true });
    const destFile = path.join(bdsBpDest, 'bedrock-voice-chat-bp.zip');
    copyFileSync(bpMcpack, destFile);
    console.log(`BDS BP -> ${destFile}`);
  }

  if (bdsRpDest && existsSync(rpMcpack)) {
    mkdirSync(bdsRpDest, { recursive: true });
    const destFile = path.join(bdsRpDest, 'bedrock-voice-chat-rp.zip');
    copyFileSync(rpMcpack, destFile);
    console.log(`BDS RP -> ${destFile}`);
  }
}

console.log('\n=== Build Complete ===');
