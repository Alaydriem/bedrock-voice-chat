import type { APIRoute } from 'astro';
import { getCollection } from 'astro:content';

/** The whole wiki as one document, for agents that prefer one request to forty. */
const SITE = 'https://www.bedrockvoicechat.com';

const ORDER = ['start', 'player', 'server', 'platforms', 'creator', 'reference'];

export const GET: APIRoute = async () => {
  const docs = (await getCollection('docs')).filter((e) => e.id.startsWith('wiki/'));

  const rank = (id: string) => {
    const rest = id.replace(/^wiki\//, '');
    if (rest === 'index') return -1;
    const i = ORDER.indexOf(rest.split('/')[0]);
    return i === -1 ? ORDER.length : i;
  };

  docs.sort((a, b) => rank(a.id) - rank(b.id) || a.id.localeCompare(b.id));

  const out: string[] = [
    '# Bedrock Voice Chat — complete documentation',
    '',
    'Proximity voice chat for Minecraft Bedrock and Java. Self-hosted: each server is',
    'run by the community that owns it.',
    '',
    `Individual pages: ${SITE}/llms.txt`,
    '',
    '---',
    '',
  ];

  for (const entry of docs) {
    const url = `${SITE}/${entry.id.replace(/\/?index$/, '')}`;
    out.push(`# ${entry.data.title}`, '');
    if (entry.data.description) out.push(`> ${entry.data.description}`, '');
    out.push(`Source: ${url}`, '', (entry.body ?? '').trim(), '', '---', '');
  }

  return new Response(out.join('\n'), {
    headers: { 'Content-Type': 'text/plain; charset=utf-8' },
  });
};
