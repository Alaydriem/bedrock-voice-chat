import { render } from "@testing-library/svelte";
import { describe, expect, it, vi } from "vitest";
import type { Plate } from "../../js/app/settings/Plate";

const { default: PlateGrid } = await import("../../components/settings/PlateGrid.svelte");

function plate(overrides: Partial<Plate> = {}): Plate {
    return {
        id: "b1",
        name: "Alaydriem's SMP",
        detail: "mc.alaydriem.com:19132",
        glyphKey: "mc.alaydriem.com",
        chips: [],
        favourite: false,
        active: false,
        reachable: true,
        readonly: false,
        ...overrides,
    };
}

function mount(plates: Plate[], props: Record<string, unknown> = {}) {
    const host = document.createElement("div");
    document.body.append(host);
    render(PlateGrid, {
        target: host,
        props: {
            plates,
            onconnect: () => {},
            onstop: () => {},
            onfavourite: () => {},
            ...props,
        },
    });
    return {
        host,
        action: () => host.querySelector<HTMLButtonElement>(".rad-server__foot .rad-btn"),
        fav: () => host.querySelector<HTMLElement>(".rad-fav"),
        edit: () => host.querySelector<HTMLElement>('[aria-label^="Edit"]'),
        remove: () => host.querySelector<HTMLElement>('[aria-label^="Remove"]'),
        add: () => host.querySelector<HTMLElement>(".rad-server-add"),
    };
}

describe("PlateGrid", () => {
    it("offers a connect on a plate that can be reached", () => {
        const onconnect = vi.fn();
        const view = mount([plate()], { onconnect });
        expect(view.action()?.textContent?.trim()).toBe("Connect");
        expect(view.action()?.disabled).toBe(false);
        view.action()?.click();
        expect(onconnect).toHaveBeenCalledWith("b1");
    });

    // A closed Realm, or a server whose operator has Bedrock support off. Offering a
    // connect that cannot succeed is worse than offering none.
    it("refuses a connect that cannot succeed", () => {
        const view = mount([plate({ reachable: false })]);
        expect(view.action()?.disabled).toBe(true);
    });

    // The one already in use offers the way out of it, not another way in.
    it("offers a stop on the plate already in use", () => {
        const onstop = vi.fn();
        const view = mount([plate({ active: true })], { onstop });
        expect(view.action()?.textContent?.trim()).toBe("Stop");
        view.action()?.click();
        expect(onstop).toHaveBeenCalledWith("b1");
    });

    it("reports the favourite state on the control itself", () => {
        const onfavourite = vi.fn();
        const view = mount([plate({ favourite: true })], { onfavourite });
        expect(view.fav()?.getAttribute("aria-pressed")).toBe("true");
        view.fav()?.click();
        expect(onfavourite).toHaveBeenCalledWith("b1");
    });

    // Operator-supplied entries come from the server's own config and are not stored
    // locally. Offering an edit that cannot persist is a control that lies.
    it("does not offer to edit or remove an operator's entry", () => {
        const view = mount([plate({ readonly: true })], {
            onedit: () => {},
            onremove: () => {},
        });
        expect(view.edit()).toBeNull();
        expect(view.remove()).toBeNull();
    });

    it("offers edit and remove on your own entry", () => {
        const view = mount([plate({ readonly: false })], {
            onedit: () => {},
            onremove: () => {},
        });
        expect(view.edit()).not.toBeNull();
        expect(view.remove()).not.toBeNull();
    });

    // Realms cannot be added by hand — they come from the account — so the tile is only
    // there where adding is a thing you can do.
    it("shows the add tile only where one was given", () => {
        expect(mount([plate()]).add()).toBeNull();
        expect(mount([plate()], { addLabel: "Add a server", onadd: () => {} }).add()).not.toBeNull();
    });

    it("renders every chip a plate carries", () => {
        const view = mount([
            plate({
                chips: [
                    { label: "Forwarding here", severity: "ok" },
                    { label: "From your server", severity: "muted" },
                ],
            }),
        ]);
        expect(view.host.textContent).toContain("Forwarding here");
        expect(view.host.textContent).toContain("From your server");
    });
});
