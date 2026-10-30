import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import NodecraftScreen from "../../components/login/NodecraftScreen.svelte";

function props(overrides: Record<string, unknown> = {}) {
    return {
        onsignup: vi.fn(),
        ontoggle: vi.fn(),
        onguide: vi.fn(),
        ondiscord: vi.fn(),
        onback: vi.fn(),
        onsignin: vi.fn(),
        ...overrides,
    };
}

describe("NodecraftScreen", () => {
    // A player who pays and only then finds a domain step asks for a refund.
    it("lists what is needed before the sign-up button", () => {
        render(NodecraftScreen, { props: props() });
        expect(screen.getByRole("checkbox", { name: /domain name/i })).toBeInTheDocument();
        expect(screen.getByRole("checkbox", { name: /cloudflare/i })).toBeInTheDocument();
        expect(screen.getByRole("checkbox", { name: /nodecraft pro server/i })).toBeInTheDocument();
    });

    // "An IP address does not work" is the detail that stops a wasted purchase; a screen
    // reader must hear it.
    it("announces each row's note with its label", () => {
        render(NodecraftScreen, { props: props() });
        expect(
            screen.getByRole("checkbox", { name: /domain name.*ip address does not work/i }),
        ).toBeInTheDocument();
    });

    it("never blocks the sign-up button on the checklist", async () => {
        const onsignup = vi.fn();
        render(NodecraftScreen, { props: props({ onsignup }) });
        const button = screen.getByRole("button", { name: /get a nodecraft server/i });
        expect(button).toBeEnabled();
        await userEvent.click(button);
        expect(onsignup).toHaveBeenCalledWith(0);
    });

    it("reports only the rows still ticked", async () => {
        const onsignup = vi.fn();
        const ontoggle = vi.fn();
        render(NodecraftScreen, { props: props({ onsignup, ontoggle }) });
        const domain = screen.getByRole("checkbox", { name: /domain name/i });
        await userEvent.click(domain);
        await userEvent.click(screen.getByRole("checkbox", { name: /cloudflare/i }));
        await userEvent.click(domain);
        expect(ontoggle).toHaveBeenNthCalledWith(1, "domain", true);
        expect(ontoggle).toHaveBeenNthCalledWith(3, "domain", false);
        expect(domain).toHaveAttribute("aria-checked", "false");
        await userEvent.click(screen.getByRole("button", { name: /get a nodecraft server/i }));
        expect(onsignup).toHaveBeenCalledWith(1);
    });

    it("toggles a row from the keyboard", async () => {
        render(NodecraftScreen, { props: props() });
        const server = screen.getByRole("checkbox", { name: /nodecraft pro server/i });
        server.focus();
        await userEvent.keyboard(" ");
        expect(server).toHaveAttribute("aria-checked", "true");
    });

    it("opens the Bedrock and Java guides", async () => {
        const onguide = vi.fn();
        render(NodecraftScreen, { props: props({ onguide }) });
        await userEvent.click(screen.getByRole("button", { name: /bedrock on nodecraft guide/i }));
        await userEvent.click(screen.getByRole("button", { name: /java guides/i }));
        expect(onguide.mock.calls).toEqual([["bedrock"], ["java"]]);
    });

    it("goes back to the other options", async () => {
        const onback = vi.fn();
        render(NodecraftScreen, { props: props({ onback }) });
        await userEvent.click(screen.getByRole("button", { name: /other options/i }));
        expect(onback).toHaveBeenCalledOnce();
    });
});
