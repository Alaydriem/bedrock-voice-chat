import { render } from "@testing-library/svelte";
import { readable } from "svelte/store";
import { describe, expect, it } from "vitest";
import type { BedrockManager } from "../../js/app/managers/bedrock/BedrockManager";

const { default: XboxSignIn } = await import("../../components/settings/XboxSignIn.svelte");

function stub(over: Record<string, unknown> = {}) {
    return {
        showLoginModal: readable(true),
        deviceCode: readable("JQ4H-7TDM"),
        deviceUrl: readable("microsoft.com/link"),
        loginError: readable(""),
        codeCopied: readable(false),
        isRestoringAuth: readable(false),
        closeLoginModal: async () => {},
        copyDeviceCode: async () => {},
        openLoginUrl: async () => {},
        ...over,
    } as unknown as BedrockManager;
}

function mount(bedrock: BedrockManager) {
    const host = document.createElement("div");
    document.body.append(host);
    render(XboxSignIn as never, { target: host, props: { bedrock } } as never);
    return host;
}

/**
 * The address was a read-only text input in a two-column row. Nobody types into it, and it
 * squeezed the code — the one thing that has to be read — into the column beside it.
 */
describe("XboxSignIn layout", () => {
    it("presents the address as a link rather than a form field", () => {
        const host = mount(stub());

        expect(host.querySelector("input")).toBeNull();
        const link = host.querySelector<HTMLElement>(".rad-link-card");
        expect(link).not.toBeNull();
        expect(link?.textContent).toContain("microsoft.com/link");
    });

    it("opens the sign-in page from the link", async () => {
        let opened = 0;
        const host = mount(stub({ openLoginUrl: async () => void (opened += 1) }));

        host.querySelector<HTMLElement>(".rad-link-card")?.click();
        await Promise.resolve();

        expect(opened).toBe(1);
    });

    it("gives the modal the roomier measure", () => {
        const host = mount(stub());

        expect(host.querySelector(".rad-modal--wide")).not.toBeNull();
    });

    it("still shows the code and offers to copy it", () => {
        const host = mount(stub());

        expect(host.querySelector(".rad-kbd")?.textContent?.trim()).toBe("JQ4H-7TDM");
        expect(host.querySelector('[aria-label^="Copy"]')).not.toBeNull();
    });
});
