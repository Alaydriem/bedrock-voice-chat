const esbuild = require('esbuild');
const path = require('path');

async function build() {
  try {
    await esbuild.build({
      entryPoints: [path.join(__dirname, 'src', 'main.ts')],
      bundle: true,
      outfile: path.join(__dirname, 'scripts', 'main.js'),
      platform: 'neutral',
      format: 'esm',
      target: 'es2021',
      external: [
        '@minecraft/server',
        '@minecraft/server-net',
        '@minecraft/server-admin',
        '@minecraft/server-ui'
      ],
      minify: false,
      sourcemap: false,
      logLevel: 'info',
    });
    console.log('Build completed successfully');
  } catch (error) {
    console.error('Build failed:', error);
    process.exit(1);
  }
}

build();
