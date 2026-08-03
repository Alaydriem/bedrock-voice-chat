import type { APIRoute, GetStaticPaths } from 'astro';
import { getCollection } from 'astro:content';

/**
 * Serves the raw markdown of every wiki page at the same URL with `.md`
 * appended: /wiki/server/tls/ also answers at /wiki/server/tls.md
 *
 * Static hosting cannot do content negotiation, so the extension is the
 * mechanism. Discovery is via /llms.txt and the `rel="alternate"` link in each
 * page's head.
 */
export const getStaticPaths: GetStaticPaths = async () => {
  const docs = await getCollection('docs');
  return docs
    .filter((entry) => entry.id.startsWith('wiki/'))
    .map((entry) => ({
      // The route already contains `/wiki/`, so the prefix is stripped here.
      params: { slug: entry.id.replace(/^wiki\//, '').replace(/\/?index$/, '') || undefined },
      props: { entry },
    }));
};

export const GET: APIRoute = ({ props }) => {
  const { entry } = props as { entry: Awaited<ReturnType<typeof getCollection>>[number] };
  const title = entry.data.title;
  const description = entry.data.description ?? '';

  const front = ['---', `title: ${JSON.stringify(title)}`];
  if (description) front.push(`description: ${JSON.stringify(description)}`);
  front.push(`source: https://www.bedrockvoicechat.com/${entry.id.replace(/\/?index$/, '')}/`);
  front.push('---', '');

  return new Response(front.join('\n') + (entry.body ?? ''), {
    headers: {
      'Content-Type': 'text/markdown; charset=utf-8',
    },
  });
};
