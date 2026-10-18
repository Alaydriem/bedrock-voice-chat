import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import UpdateOfferDialog from "../../components/shell/UpdateOfferDialog.svelte";

function props(overrides: Record<string, unknown> = {}) {
    return { open: true, onaccept: vi.fn(), ondismiss: vi.fn(), ...overrides };
}

describe("UpdateOfferDialog", () => {
    it("offers the update", () => {
        render(UpdateOfferDialog, { props: props() });
        expect(screen.getByRole("dialog", { name: "Update available" })).toHaveAttribute(
            "aria-hidden",
            "false",
        );
    });

    it("installs from Update now", async () => {
        const onaccept = vi.fn();
        render(UpdateOfferDialog, { props: props({ onaccept }) });
        await userEvent.click(screen.getByRole("button", { name: "Update now" }));
        expect(onaccept).toHaveBeenCalledOnce();
    });

    it("can be put off", async () => {
        const ondismiss = vi.fn();
        render(UpdateOfferDialog, { props: props({ ondismiss }) });
        await userEvent.click(screen.getByRole("button", { name: "Not now" }));
        expect(ondismiss).toHaveBeenCalledOnce();
    });

    it("stays out of the way when there is nothing to offer", () => {
        render(UpdateOfferDialog, { props: props({ open: false }) });
        expect(screen.getByRole("dialog", { hidden: true })).toHaveAttribute(
            "aria-hidden",
            "true",
        );
    });
});
