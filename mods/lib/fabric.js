// Fabric dev-server provisioning, used by `dev-server.js --fabric`.
//
// The server launcher comes from Fabric's meta API; Fabric API, Geyser, Floodgate and Simple
// Voice Chat from Modrinth. The launcher downloads the vanilla server on its first start.

const { Download } = require('./download');
const { JavaServer } = require('./java-server');
const { Modrinth } = require('./modrinth');

const MC_VERSION = '26.3';
const META_API = 'https://meta.fabricmc.net/v2/versions';

class Fabric {
  // FABRIC_MC_VERSION overrides the pinned Minecraft version.
  static minecraft(env) {
    return env.FABRIC_MC_VERSION || MC_VERSION;
  }

  static async provision(serverDir, env, update) {
    const mc = Fabric.minecraft(env);
    await JavaServer.provision(serverDir, mc, () => Fabric.resolve(mc), update);
    JavaServer.acceptEula(serverDir);
  }

  static async resolve(mc) {
    const mods = ['fabric-api', 'geyser', 'floodgate', 'simple-voice-chat'];
    const [server, ...resolved] = await Promise.all([
      Fabric.server(mc),
      ...mods.map((p) => Modrinth.latest(p, 'fabric', mc)),
    ]);
    const artifacts = { server: { ...server, dir: '' } };
    mods.forEach((p, i) => {
      artifacts[p] = { ...resolved[i], dir: 'mods' };
    });
    return artifacts;
  }

  // The server launcher for the newest stable loader and installer. Fabric publishes no checksum
  // for the launcher jar.
  static async server(mc) {
    const [loaders, installers] = await Promise.all([
      Download.json(`${META_API}/loader/${mc}`),
      Download.json(`${META_API}/installer`),
    ]);
    const loader = (loaders.find((l) => l.loader.stable) || loaders[0])?.loader.version;
    const installer = (installers.find((i) => i.stable) || installers[0])?.version;
    if (!loader || !installer) {
      throw new Error(`Fabric meta has no loader or installer for Minecraft ${mc}`);
    }
    return {
      name: `fabric loader ${loader}`,
      url: `${META_API}/loader/${mc}/${loader}/${installer}/server/jar`,
      filename: `fabric-server-mc.${mc}-loader.${loader}-launcher.${installer}.jar`,
      hash: null,
    };
  }
}

module.exports = { Fabric };
