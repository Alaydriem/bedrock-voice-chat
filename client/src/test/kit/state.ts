/**
 * `$app/state`, outside SvelteKit.
 *
 * A plain object rather than a rune: a test that wants a different URL assigns to
 * `page.url`, and the components reading it are re-rendered by the test rather than by
 * SvelteKit's navigation.
 */
export const page = {
    url: new URL("http://localhost/"),
    params: {} as Record<string, string>,
    route: { id: null as string | null },
    status: 200,
    error: null as Error | null,
    data: {} as Record<string, unknown>,
    form: null as unknown,
};

export const navigating = null;
export const updated = { current: false };
