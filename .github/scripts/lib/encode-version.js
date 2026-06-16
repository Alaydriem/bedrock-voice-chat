/**
 * Shared monotonic version encoder for mod manifests.
 *
 * Encoding: major.minor.(patch*1000 + channel*100 + prerelease)
 * Channels: 1=alpha, 2=internal, 5=beta, 8=rc, 9=stable
 *
 * Used by .github/scripts/patch-mod-versions.js and the BDS build (mods/bds)
 * so both produce identical version arrays from a single source of truth.
 */
class VersionEncoder {
  /**
   * Encode a semantic version into the monotonic 3-component form.
   * @param {string} version - e.g. "1.2.3" or "1.2.3-beta.1"
   * @returns {{ major: number, minor: number, encodedPatch: number, display: string, array: [number, number, number] }}
   */
  static encode(version) {
    const [core, prerelease] = version.split('-');
    const [major = 0, minor = 0, patch = 0] = core.split('.').map(Number);

    let channel = 9;
    let prereleaseNum = 0;

    if (prerelease) {
      const match = prerelease.match(/^(alpha|internal|beta|rc)\.?(\d+)?$/);
      if (match) {
        const channelName = match[1];
        prereleaseNum = parseInt(match[2]) || 1;

        if (channelName === 'alpha') channel = 1;
        else if (channelName === 'internal') channel = 2;
        else if (channelName === 'beta') channel = 5;
        else if (channelName === 'rc') channel = 8;
      }
    }

    const encodedPatch = patch * 1000 + channel * 100 + prereleaseNum;
    const display = `${major}.${minor}.${encodedPatch}`;

    return { major, minor, encodedPatch, display, array: [major, minor, encodedPatch] };
  }

  static versionCode(version) {
    const { major, minor, encodedPatch } = VersionEncoder.encode(version);
    return major * 1000000 + minor * 10000 + encodedPatch;
  }

  /**
   * Release channel for a semver, derived from its prerelease tag. Mirrors
   * encode(): an absent or unrecognized prerelease is treated as stable.
   * @param {string} version - e.g. "1.2.3" or "1.2.3-beta.1"
   * @returns {{ name: string, number: number }}
   */
  static channel(version) {
    const [, prerelease] = version.split('-');
    if (!prerelease) return { name: 'stable', number: 9 };
    const match = prerelease.match(/^(alpha|internal|beta|rc)\.?(\d+)?$/);
    if (!match) return { name: 'stable', number: 9 };
    const numbers = { alpha: 1, internal: 2, beta: 5, rc: 8 };
    return { name: match[1], number: numbers[match[1]] };
  }

  /**
   * Operator-controlled channel -> Flagsmith environment mapping. Store track
   * is irrelevant; the release channel decides which environment a build's
   * feature flags resolve against.
   * @param {string} version
   * @returns {string} one of "dev" | "staging" | "prod"
   */
  static flagsmithEnvironment(version) {
    const name = VersionEncoder.channel(version).name;
    if (name === 'alpha' || name === 'internal') return 'dev';
    if (name === 'beta' || name === 'rc') return 'staging';
    return 'prod';
  }
}

module.exports = { VersionEncoder };
