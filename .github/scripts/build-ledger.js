'use strict';

class BuildLedger {
  static record(buildNumber, meta) {
    return {
      build_number: Number(buildNumber),
      sha: meta.sha || '',
      branch: meta.branch || '',
      ref: meta.ref || '',
      version: meta.version || '',
      track: meta.track || '',
      run: meta.run || '',
      ts: meta.ts || '',
    };
  }

  static toJson(recordsByNumber) {
    const builds = Object.values(recordsByNumber || {})
      .map((v) => (typeof v === 'string' ? JSON.parse(v) : v))
      .sort((a, b) => b.build_number - a.build_number);
    return JSON.stringify({ builds }, null, 2) + '\n';
  }
}

module.exports = { BuildLedger };
