// HTTP helpers shared by the dev-server provisioners.
//
// Errors throw rather than exit, as in `bds.js`.

const { createHash } = require('crypto');
const { createReadStream, createWriteStream, renameSync, rmSync } = require('fs');
const { Readable } = require('stream');
const { pipeline } = require('stream/promises');

// Modrinth rejects requests without a User-Agent and asks for one that names the client.
const USER_AGENT = 'bvc-dev-server/1.0';

class Download {
  static async json(url) {
    const res = await fetch(url, { headers: { 'User-Agent': USER_AGENT } });
    if (res.status !== 200) {
      throw new Error(`GET ${url} failed: HTTP ${res.status}`);
    }
    return res.json();
  }

  // Downloads `url` to `dest`. When `hash` is given as { algorithm, value }, a file whose digest
  // differs is deleted and the download fails.
  static async file(url, dest, hash) {
    console.log(`[dev-server] downloading ${url}`);
    const res = await fetch(url, { headers: { 'User-Agent': USER_AGENT } });
    if (res.status !== 200 || !res.body) {
      throw new Error(`Download failed: HTTP ${res.status} from ${url}`);
    }
    // A partial download must not be mistaken for a finished one on the next run.
    const partial = `${dest}.part`;
    await pipeline(Readable.fromWeb(res.body), createWriteStream(partial));
    if (hash) {
      const actual = await Download.digest(partial, hash.algorithm);
      if (actual !== hash.value.toLowerCase()) {
        rmSync(partial);
        throw new Error(
          `${hash.algorithm} mismatch for ${url}\n  expected ${hash.value}\n  got      ${actual}`
        );
      }
    }
    renameSync(partial, dest);
  }

  static async digest(file, algorithm) {
    const h = createHash(algorithm);
    await pipeline(createReadStream(file), h);
    return h.digest('hex');
  }
}

module.exports = { Download };
