'use strict';
const fs = require('fs');
const { BuildLedger } = require('./build-ledger');

class LedgerPublisher {
  static async run() {
    const [redisUrl, redisToken, outPath] = process.argv.slice(2);
    const res = await fetch(`${redisUrl}/hgetall/builds`, {
      headers: { Authorization: `Bearer ${redisToken}` },
    });
    if (!res.ok) throw new Error(`HGETALL failed: ${res.status}`);
    const body = await res.json();
    // Upstash HGETALL returns a flat [field, value, field, value, ...] array.
    const flat = body.result || [];
    const map = {};
    for (let i = 0; i < flat.length; i += 2) map[flat[i]] = flat[i + 1];
    fs.writeFileSync(outPath, BuildLedger.toJson(map));
    console.log(`Wrote ${Object.keys(map).length} builds to ${outPath}`);
  }
}

LedgerPublisher.run().catch((e) => { console.error(e); process.exit(1); });
