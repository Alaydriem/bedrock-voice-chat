/**
 * `$app/navigation`, outside SvelteKit.
 *
 * Vitest runs the Svelte plugin, not the SvelteKit one, so nothing supplies these. Without
 * them a route component cannot even be imported — which is also why the coverage sweep
 * fell back to parsing raw `.svelte` source and dropped every route from the report.
 */
export const navigations: string[] = [];

export async function goto(url: string | URL): Promise<void> {
    navigations.push(String(url));
}

export function invalidateAll(): Promise<void> {
    return Promise.resolve();
}
