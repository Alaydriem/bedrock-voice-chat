// Paper dev-server provisioning, used by `dev-server.js --paper`.
//
// Paper comes from PaperMC's Fill API; Geyser and Simple Voice Chat from Modrinth. Floodgate's
// Spigot build is not published to Modrinth, so it comes from GeyserMC's download API.

const { Download } = require('./download');
const { JavaServer } = require('./java-server');
const { Modrinth } = require('./modrinth');

const MC_VERSION = '26.3';
const FILL_API = 'https://fill.papermc.io/v3/projects/paper';
const FLOODGATE_BUILD = 'https://download.geysermc.org/v2/projects/floodgate/versions/latest/builds/latest';

class Paper {
  // PAPER_MC_VERSION overrides the pinned Minecraft version.
  static minecraft(env) {
    return env.PAPER_MC_VERSION || MC_VERSION;
  }

  static async provision(serverDir, env, update) {
    const mc = Paper.minecraft(env);
    await JavaServer.provision(serverDir, mc, () => Paper.resolve(mc), update);
    JavaServer.acceptEula(serverDir);
  }

  static async resolve(mc) {
    const [server, geyser, floodgate, svc] = await Promise.all([
      Paper.server(mc),
      Modrinth.latest('geyser', 'paper', mc),
      Paper.floodgate(),
      Modrinth.latest('simple-voice-chat', 'paper', mc),
    ]);
    return {
      server: { ...server, dir: '' },
      geyser: { ...geyser, dir: 'plugins' },
      floodgate: { ...floodgate, dir: 'plugins' },
      'simple-voice-chat': { ...svc, dir: 'plugins' },
    };
  }

  // The newest STABLE Paper build for `mc`.
  static async server(mc) {
    const builds = await Download.json(`${FILL_API}/versions/${mc}/builds`);
    const build = builds.find((b) => b.channel === 'STABLE');
    if (!build) throw new Error(`PaperMC has no stable build for Minecraft ${mc}`);
    const d = build.downloads['server:default'];
    return {
      name: `paper ${mc}-${build.id}`,
      url: d.url,
      filename: d.name,
      hash: { algorithm: 'sha256', value: d.checksums.sha256 },
    };
  }

  // GeyserMC names every Floodgate jar `floodgate-spigot.jar`, so the build number goes into the
  // filename to make an update replace it.
  static async floodgate() {
    const b = await Download.json(FLOODGATE_BUILD);
    const d = b.downloads.spigot;
    return {
      name: `floodgate ${b.version}-b${b.build}`,
      url: `https://download.geysermc.org/v2/projects/floodgate/versions/${b.version}/builds/${b.build}/downloads/spigot`,
      filename: `floodgate-spigot-${b.version}-b${b.build}.jar`,
      hash: { algorithm: 'sha256', value: d.sha256 },
    };
  }
}

module.exports = { Paper };
