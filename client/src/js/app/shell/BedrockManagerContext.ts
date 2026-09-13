/**
 * The session's one `BedrockManagerHolder`, published by the dashboard layout.
 *
 * Context rather than a module singleton: the settings screen is a descendant route of the
 * dashboard, so the layout can hand the same holder down without it becoming reachable from
 * anywhere in the app. The standalone settings route has no dashboard behind it and builds
 * its own, which is correct — that route has no session to outlive.
 */
export const BEDROCK_MANAGER_KEY = Symbol('bvc:bedrock-manager');
