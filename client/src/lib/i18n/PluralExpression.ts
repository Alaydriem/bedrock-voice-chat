interface ParseState {
  readonly tokens: readonly string[];
  position: number;
  readonly n: number;
}

/**
 * Evaluates the C expression carried in a gettext `Plural-Forms` header.
 *
 * The grammar is deliberately smaller than C's: no arithmetic beyond `%`, no unary
 * operators, no identifiers other than `n`. Expressions reach this evaluator from `.po`
 * files written by outside contributors, so anything the grammar cannot express is
 * rejected rather than interpreted.
 */
export default class PluralExpression {
  static evaluate(source: string, n: number): number {
    const tokens = PluralExpression.#tokenize(source.replace(/;\s*$/, ""));
    const state: ParseState = { tokens, position: 0, n };
    const value = PluralExpression.#ternary(state);

    if (state.position !== tokens.length) {
      throw new Error(`Trailing tokens in plural expression: ${source}`);
    }
    return value;
  }

  static readonly #TOKEN = /\d+|n|\|\||&&|<=|>=|==|!=|[<>%?:()]/g;

  static #tokenize(source: string): string[] {
    const tokens: string[] = [];
    let cursor = 0;

    for (const match of source.matchAll(PluralExpression.#TOKEN)) {
      if (source.slice(cursor, match.index).trim() !== "") {
        throw new Error(`Illegal token in plural expression: ${source}`);
      }
      tokens.push(match[0]);
      cursor = match.index + match[0].length;
    }

    if (source.slice(cursor).trim() !== "") {
      throw new Error(`Illegal token in plural expression: ${source}`);
    }
    return tokens;
  }

  static #ternary(state: ParseState): number {
    const condition = PluralExpression.#or(state);
    if (state.tokens[state.position] !== "?") return condition;

    state.position += 1;
    const whenTrue = PluralExpression.#ternary(state);
    PluralExpression.#expect(state, ":");
    const whenFalse = PluralExpression.#ternary(state);
    return condition !== 0 ? whenTrue : whenFalse;
  }

  static #or(state: ParseState): number {
    let left = PluralExpression.#and(state);
    while (state.tokens[state.position] === "||") {
      state.position += 1;
      const right = PluralExpression.#and(state);
      left = left !== 0 || right !== 0 ? 1 : 0;
    }
    return left;
  }

  static #and(state: ParseState): number {
    let left = PluralExpression.#equality(state);
    while (state.tokens[state.position] === "&&") {
      state.position += 1;
      const right = PluralExpression.#equality(state);
      left = left !== 0 && right !== 0 ? 1 : 0;
    }
    return left;
  }

  static #equality(state: ParseState): number {
    let left = PluralExpression.#relational(state);
    while (state.tokens[state.position] === "==" || state.tokens[state.position] === "!=") {
      const operator = state.tokens[state.position];
      state.position += 1;
      const right = PluralExpression.#relational(state);
      left = (operator === "==" ? left === right : left !== right) ? 1 : 0;
    }
    return left;
  }

  static readonly #RELATIONAL = ["<", ">", "<=", ">="];

  static #relational(state: ParseState): number {
    let left = PluralExpression.#modulo(state);
    while (PluralExpression.#RELATIONAL.includes(state.tokens[state.position])) {
      const operator = state.tokens[state.position];
      state.position += 1;
      const right = PluralExpression.#modulo(state);
      left = PluralExpression.#compare(operator, left, right) ? 1 : 0;
    }
    return left;
  }

  static #compare(operator: string, left: number, right: number): boolean {
    switch (operator) {
      case "<":
        return left < right;
      case ">":
        return left > right;
      case "<=":
        return left <= right;
      default:
        return left >= right;
    }
  }

  static #modulo(state: ParseState): number {
    let left = PluralExpression.#primary(state);
    while (state.tokens[state.position] === "%") {
      state.position += 1;
      left = left % PluralExpression.#primary(state);
    }
    return left;
  }

  static #primary(state: ParseState): number {
    const token = state.tokens[state.position];
    if (token === undefined) {
      throw new Error("Unexpected end of plural expression");
    }

    if (token === "(") {
      state.position += 1;
      const value = PluralExpression.#ternary(state);
      PluralExpression.#expect(state, ")");
      return value;
    }

    state.position += 1;
    if (token === "n") return state.n;
    if (/^\d+$/.test(token)) return Number(token);
    throw new Error(`Unexpected token "${token}" in plural expression`);
  }

  static #expect(state: ParseState, token: string): void {
    if (state.tokens[state.position] !== token) {
      throw new Error(`Expected "${token}" in plural expression`);
    }
    state.position += 1;
  }
}
