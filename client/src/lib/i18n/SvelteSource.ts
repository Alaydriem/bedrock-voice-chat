import { parse } from "svelte/compiler";

interface Span {
  start: number;
  end: number;
}

/**
 * Recovers the TypeScript inside a Svelte component as a buffer whose line numbers match
 * the original file.
 *
 * Line alignment is what lets the extractor emit accurate `#:` references without a source
 * map, and accurate references are most of what a translator has to work with.
 */
export default class SvelteSource {
  static toTypeScript(source: string): string {
    const root = parse(source, { modern: true }) as Record<string, unknown>;
    const lines: string[] = new Array(source.split("\n").length).fill("");

    for (const key of ["module", "instance"]) {
      const block = root[key] as { content?: Span } | undefined;
      const content = block?.content;
      if (content === undefined) continue;
      SvelteSource.#place(lines, source, content.start, source.slice(content.start, content.end));
    }

    for (const expression of SvelteSource.#expressionTags(root.fragment)) {
      const text = source.slice(expression.start, expression.end);
      SvelteSource.#place(lines, source, expression.start, `;(${text});`);
    }

    return lines.join("\n");
  }

  static #place(lines: string[], source: string, offset: number, text: string): void {
    const first = SvelteSource.#lineAt(source, offset);

    text.split("\n").forEach((line, index) => {
      const target = first + index;
      if (target < lines.length) {
        lines[target] = lines[target] === "" ? line : `${lines[target]} ${line}`;
      }
    });
  }

  static #lineAt(source: string, offset: number): number {
    let line = 0;
    for (let index = 0; index < offset; index += 1) {
      if (source[index] === "\n") line += 1;
    }
    return line;
  }

  // A structural walk rather than a typed one. The template AST changes shape across
  // Svelte minor versions; the node type and its expression span do not.
  static #expressionTags(node: unknown, found: Span[] = []): Span[] {
    if (node === null || typeof node !== "object") return found;

    if (Array.isArray(node)) {
      for (const child of node) SvelteSource.#expressionTags(child, found);
      return found;
    }

    const record = node as Record<string, unknown>;
    if (record.type === "ExpressionTag" && record.expression != null) {
      found.push(record.expression as Span);
    }

    for (const key of Object.keys(record)) {
      if (key === "parent") continue;
      SvelteSource.#expressionTags(record[key], found);
    }
    return found;
  }
}
