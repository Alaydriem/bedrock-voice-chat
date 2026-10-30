import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import NoServerScreen from "../../components/login/NoServerScreen.svelte";

function props(overrides: Record<string, unknown> = {}) {
    return {
        onnodecraft: vi.fn(),
        onguide: vi.fn(),
        oncopyinvite: vi.fn(),
        onsite: vi.fn(),
        onwiki: vi.fn(),
        ondiscord: vi.fn(),
        onsignin: vi.fn(),
        onintro: vi.fn(),
        ...overrides,
    };
}

describe("NoServerScreen", () => {
    it("explains what is involved before asking for a decision", () => {
        render(NoServerScreen, { props: props() });
        expect(screen.getByText(/run the bvc server/i)).toBeInTheDocument();
        expect(screen.getByText(/add the mod to your world/i)).toBeInTheDocument();
    });

    it("leads with the Nodecraft offer", async () => {
        const onnodecraft = vi.fn();
        render(NoServerScreen, { props: props({ onnodecraft }) });
        expect(screen.getByText(/30% off your first month/i)).toBeInTheDocument();
        await userEvent.click(screen.getByRole("button", { name: /set up on nodecraft/i }));
        expect(onnodecraft).toHaveBeenCalledOnce();
    });

    it("keeps the self-host and friend routes, and drops 'just looking'", () => {
        render(NoServerScreen, { props: props() });
        expect(screen.getByText(/i want to run my own/i)).toBeInTheDocument();
        expect(screen.getByText(/a friend runs the server/i)).toBeInTheDocument();
        expect(screen.queryByText(/i'm just looking/i)).toBeNull();
    });

    it("opens the install guide for someone running their own", async () => {
        const onguide = vi.fn();
        render(NoServerScreen, { props: props({ onguide }) });
        await userEvent.click(screen.getByText(/i want to run my own/i));
        expect(onguide).toHaveBeenCalledOnce();
    });

    it("goes back to the introduction", async () => {
        const onintro = vi.fn();
        render(NoServerScreen, { props: props({ onintro }) });
        await userEvent.click(screen.getByRole("button", { name: /introduction/i }));
        expect(onintro).toHaveBeenCalledOnce();
    });

    it("copies an invite when the friend route is taken", async () => {
        const oncopyinvite = vi.fn();
        render(NoServerScreen, { props: props({ oncopyinvite }) });
        await userEvent.click(screen.getByText(/a friend runs the server/i));
        expect(oncopyinvite).toHaveBeenCalledOnce();
    });

    // Someone who got here by mistake, or who found an address meanwhile, must not be
    // stuck.
    it("keeps a way back to sign in", async () => {
        const onsignin = vi.fn();
        render(NoServerScreen, { props: props({ onsignin }) });
        await userEvent.click(screen.getByRole("button", { name: /i have an address/i }));
        expect(onsignin).toHaveBeenCalledOnce();
    });

    it("offers the wiki and Discord without making them the main route", async () => {
        const onwiki = vi.fn();
        const ondiscord = vi.fn();
        render(NoServerScreen, { props: props({ onwiki, ondiscord }) });
        await userEvent.click(screen.getByRole("button", { name: /^wiki/i }));
        await userEvent.click(screen.getByRole("button", { name: /^discord/i }));
        expect(onwiki).toHaveBeenCalledOnce();
        expect(ondiscord).toHaveBeenCalledOnce();
    });
});
