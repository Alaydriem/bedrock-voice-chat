/**
 * Shared monotonic version encoder for mod manifests.
 *
 * Encoding: major.minor.(patch*1000 + channel*100 + prerelease)
 * Channels: 1=alpha, 5=beta, 8=rc, 9=stable
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
      const match = prerelease.match(/^(alpha|beta|rc)\.?(\d+)?$/);
      if (match) {
        const channelName = match[1];
        prereleaseNum = parseInt(match[2]) || 1;

        if (channelName === 'alpha') channel = 1;
        else if (channelName === 'beta') channel = 5;
        else if (channelName === 'rc') channel = 8;
      }
    }

    const encodedPatch = patch * 1000 + channel * 100 + prereleaseNum;
    const display = `${major}.${minor}.${encodedPatch}`;

    return { major, minor, encodedPatch, display, array: [major, minor, encodedPatch] };
  }
}

module.exports = { VersionEncoder };
