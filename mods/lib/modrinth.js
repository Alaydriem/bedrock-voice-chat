// Modrinth API v2 lookups. Read endpoints need no token or scope.

const { Download } = require('./download');

const API = 'https://api.modrinth.com/v2';

class Modrinth {
  // The newest release or beta of `project` for `loader` and `gameVersion`, as an artifact
  // { name, url, filename, hash }. Alpha builds are skipped. Several mods used here publish
  // only betas for a new Minecraft release.
  static async latest(project, loader, gameVersion) {
    const query = new URLSearchParams({
      loaders: JSON.stringify([loader]),
      game_versions: JSON.stringify([gameVersion]),
    });
    const versions = await Download.json(`${API}/project/${project}/version?${query}`);
    const version = versions.find((v) => v.version_type !== 'alpha');
    if (!version) {
      throw new Error(`Modrinth has no ${loader} build of ${project} for Minecraft ${gameVersion}`);
    }
    const file = version.files.find((f) => f.primary) || version.files[0];
    return {
      name: `${project} ${version.version_number}`,
      url: file.url,
      filename: file.filename,
      hash: { algorithm: 'sha512', value: file.hashes.sha512 },
    };
  }
}

module.exports = { Modrinth };
