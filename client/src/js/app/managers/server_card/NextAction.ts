export type NextAction =
    | { kind: 'navigate'; href: string }
    | { kind: 'none' };
