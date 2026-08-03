import type { APIRoute } from 'astro';
import { getCollection } from 'astro:content';

/**
 * The llms.txt convention: a markdown index an agent can read to find every
 * page and its raw-markdown URL, without parsing rendered HTML.
 *
 * Paired with /llms-full.txt, which inlines the whole corpus for agents that
 * would rather make one request than forty.
 */
const SITE = 'https://www.bedrockvoicechat.com';

const GROUPS: Array<[string, string]> = [
  ['start', 'Start here'],
  ['player', 'For players'],
  ['server', 'Running a server'],
  ['platforms', 'Where BVC works'],
  ['creator', 'Streaming and recording'],
  ['reference', 'Reference'],
];

function urlFor(id: string): string {
  return `${SITE}/${id.replace(/\/?index$/, '')}`;
}

export const GET: APIRoute = async () => {
  const docs = (await getCollection('docs')).filter((e) => e.id.startsWith('wiki/'));
  const byGroup = new Map<string, typeof docs>();
  let root: (typeof docs)[number] | undefined;

  for (const entry of docs) {
    const rest = entry.id.replace(/^wiki\//, '');
    if (rest === 'index') {
      root = entry;
      continue;
    }
    const group = rest.split('/')[0];
    if (!byGroup.has(group)) byGroup.set(group, [] as unknown as typeof docs);
    byGroup.get(group)!.push(entry);
  }

  const out: string[] = [
    '# Bedrock Voice Chat',
    '',
    '> Proximity voice chat for Minecraft Bedrock and Java. Players hear each other',
    '> based on where they are in the world, across platforms and devices including',
    '> consoles. Self-hosted: each server is run by the community that owns it.',
    '',
    'Every page below is also available as raw markdown by appending `.md` to its',
    'URL. `/llms-full.txt` contains the entire wiki in one document.',
    '',
  ];

  if (root) {
    out.push('## Overview', '', `- [${root.data.title}](${urlFor(root.id)}.md): ${root.data.description ?? ''}`, '');
  }

  for (const [dir, label] of GROUPS) {
    const entries = byGroup.get(dir);
    if (!entries?.length) continue;
    out.push(`## ${label}`, '');
    for (const entry of entries.sort((a, b) => a.id.localeCompare(b.id))) {
      const desc = entry.data.description ? `: ${entry.data.description}` : '';
      out.push(`- [${entry.data.title}](${urlFor(entry.id)}.md)${desc}`);
    }
    out.push('');
  }

  out.push(
    '## Other resources',
    '',
    `- [HTTP API reference](${SITE}/api): OpenAPI spec, generated from the server source`,
    `- [WebSocket API reference](${SITE}/websocket): the client's local control API`,
    '- [Source](https://github.com/Alaydriem/bedrock-voice-chat)',
    '',
  );

  return new Response(out.join('\n'), {
    headers: { 'Content-Type': 'text/plain; charset=utf-8' },
  });
};
