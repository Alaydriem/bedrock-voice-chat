import { beforeEach, describe, expect, it, vi } from "vitest";
import { DeepLinkRouter } from "../../js/app/deepLinkRouter";

/**
 * A deep-link callback carries a single-use OAuth authorization code. Redeeming it twice
 * spends the code on the first attempt and fails the second, which the user sees as a
 * failed login after a successful Microsoft sign-in.
 *
 * The delivery path makes that easy to do by accident: the live `deep-link-received`
 * event, `processPending()`, and every manager on the screen can each present the same
 * URL. The login page constructs two managers, so this is not hypothetical.
 */

function fakeStore(pending?: string) {
  const data = new Map<string, unknown>();
  if (pending) data.set("pending_deep_link", pending);
  return {
    get: vi.fn(async (key: string) => data.get(key)),
    set: vi.fn(async (key: string, value: unknown) => void data.set(key, value)),
    delete: vi.fn(async (key: string) => void data.delete(key)),
    save: vi.fn(async () => {}),
    has: vi.fn(async (key: string) => data.has(key)),
  };
}

// A URL no other test uses, so the never-cleared set cannot leak a redemption between
// them. Unique per run for the same reason.
let seq = 0;
function authUrl(): string {
  seq += 1;
  return `bedrock-voice-chat://auth?code=AUTH_CODE_${seq}&state=STATE_${seq}`;
}

describe("DeepLinkRouter redemption is once per URL", () => {
  let url: string;

  beforeEach(() => {
    url = authUrl();
  });

  it("routes a URL the first time it is presented", async () => {
    const store = fakeStore();
    const router = new DeepLinkRouter(store as never);
    const handler = vi.fn(async () => "handled" as const);
    // Replace the real handlers: this is about the routing decision, not about what an
    // auth callback then does with the code.
    (router as never as { handlers: unknown[] }).handlers = [
      { canHandle: () => true, handle: handler },
    ];

    await router.route(url);
    expect(handler).toHaveBeenCalledTimes(1);
  });

  /**
   * The regression that mattered: two routers, because two managers each built one. A
   * per-instance record cannot see what the other already redeemed.
   */
  it("does not redeem the same URL through a second router", async () => {
    const first = new DeepLinkRouter(fakeStore() as never);
    const second = new DeepLinkRouter(fakeStore() as never);

    const firstHandler = vi.fn(async () => "handled" as const);
    const secondHandler = vi.fn(async () => "handled" as const);
    (first as never as { handlers: unknown[] }).handlers = [
      { canHandle: () => true, handle: firstHandler },
    ];
    (second as never as { handlers: unknown[] }).handlers = [
      { canHandle: () => true, handle: secondHandler },
    ];

    await first.route(url);
    await second.route(url);

    expect(firstHandler).toHaveBeenCalledTimes(1);
    expect(secondHandler).not.toHaveBeenCalled();
  });

  it("does not redeem again when a pending entry repeats a routed URL", async () => {
    const store = fakeStore(url);
    const router = new DeepLinkRouter(store as never);
    const handler = vi.fn(async () => "handled" as const);
    (router as never as { handlers: unknown[] }).handlers = [
      { canHandle: () => true, handle: handler },
    ];

    await router.route(url);
    await router.processPending();

    expect(handler).toHaveBeenCalledTimes(1);
  });

  it("keeps distinct callbacks independent", async () => {
    const router = new DeepLinkRouter(fakeStore() as never);
    const handler = vi.fn(async () => "handled" as const);
    (router as never as { handlers: unknown[] }).handlers = [
      { canHandle: () => true, handle: handler },
    ];

    await router.route(url);
    await router.route(authUrl());

    expect(handler).toHaveBeenCalledTimes(2);
  });
});
