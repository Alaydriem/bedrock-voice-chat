import { fireEvent, render, waitFor } from "@testing-library/svelte";
import { beforeEach, describe, expect, it } from "vitest";
import { invokeCalls, mockInvoke } from "../tauri";

const { default: ManagePlayersPane } = await import(
    "../../components/settings/panes/ManagePlayersPane.svelte"
);

function page(items: unknown[], total = items.length) {
    return { items, total, page: 0, page_size: 8 };
}

function user(overrides: Record<string, unknown> = {}) {
    return {
        gamertag: "Bob",
        game: "minecraft",
        banished: false,
        connected: false,
        permissions: [],
        created_at: 1_753_732_440,
        ...overrides,
    };
}

function mount() {
    const host = document.createElement("div");
    document.body.append(host);
    render(ManagePlayersPane as never, { target: host } as never);
    return {
        host,
        text: () => document.body.textContent ?? "",
        button: (label: string) =>
            document.body.querySelector<HTMLButtonElement>(`[aria-label^="${label}"]`),
        byText: (label: string) =>
            [...document.body.querySelectorAll<HTMLButtonElement>("button")].find(
                (b) => b.textContent?.trim() === label,
            ),
        strips: () =>
            [...document.body.querySelectorAll<HTMLElement>(".rad-table .rad-matrix__blocks")],
        blocks: () =>
            [...document.body.querySelectorAll<HTMLElement>(".rad-table .rad-matrix__blocks i")].map(
                (block) => block.style.background,
            ),
        cards: () => [...document.body.querySelectorAll<HTMLElement>(".rad-datacard")],
        cardText: () =>
            [...document.body.querySelectorAll<HTMLElement>(".rad-datacard")].map(
                (card) => card.textContent?.replace(/\s+/g, " ").trim() ?? "",
            ),
    };
}

describe("ManagePlayersPane", () => {
    beforeEach(() => {
        document.body.innerHTML = "";
    });

    it("lists the roster the server returns", async () => {
        mockInvoke({
            api_introspect: () => ({ permissions: ["admin"] }),
            get_credential: () => JSON.stringify({ allowed: ["admin"] }),
            admin_list_users: () => page([user(), user({ gamertag: "Carol" })]),
        });

        const pane = mount();
        await waitFor(() => expect(pane.text()).toContain("Bob"));
        expect(pane.text()).toContain("Carol");
    });

    // Banning revokes a certificate and closes a live session. A misclick must not do that.
    it("asks before banning", async () => {
        mockInvoke({
            api_introspect: () => ({ permissions: ["admin"] }),
            get_credential: () => JSON.stringify({ allowed: ["admin"] }),
            admin_list_users: () => page([user()]),
            admin_set_banished: () => "applied",
        });

        const pane = mount();
        await waitFor(() => expect(pane.text()).toContain("Bob"));

        await fireEvent.click(pane.button("Ban Bob")!);
        expect(invokeCalls().some((call) => call.cmd === "admin_set_banished")).toBe(false);

        await fireEvent.click(pane.byText("Ban")!);
        await waitFor(() =>
            expect(invokeCalls().some((call) => call.cmd === "admin_set_banished")).toBe(true),
        );
    });

    // The button is hidden on the operator's own row, so this covers the other way the
    // server can answer 409: the row was theirs under an identity the pane had not resolved
    // yet, or the certificate changed underneath it.
    it("says why a refused ban was refused", async () => {
        mockInvoke({
            api_introspect: () => ({ permissions: ["admin"] }),
            get_credential: () => JSON.stringify({ allowed: ["admin"] }),
            admin_list_users: () => page([user({ gamertag: "RootAdmin" })]),
            admin_set_banished: () => "conflict",
        });

        const pane = mount();
        await waitFor(() => expect(pane.text()).toContain("RootAdmin"));

        await fireEvent.click(pane.button("Ban RootAdmin")!);
        await fireEvent.click(pane.byText("Ban")!);

        await waitFor(() => expect(pane.text()).toContain("You cannot ban yourself."));
    });

    // Adding used to drop a field into the card, which is not how anything else in the app
    // asks for a value. It is a dialog now, like the proxy editor and every confirm.
    it("asks for the gamertag in a dialog rather than in the card", async () => {
        mockInvoke({
            api_introspect: () => ({ permissions: ["admin"] }),
            get_credential: () => JSON.stringify({ allowed: ["admin"] }),
            admin_list_users: () => page([user()]),
        });

        const pane = mount();
        await waitFor(() => expect(pane.text()).toContain("Bob"));

        // Nothing to type into until it is asked for.
        expect(document.body.querySelector('[aria-label="Gamertag"]')).toBeNull();

        await fireEvent.click(pane.byText("Add player")!);

        const field = document.body.querySelector<HTMLInputElement>('[aria-label="Gamertag"]');
        expect(field).not.toBeNull();
        expect(field!.closest(".rad-modal")).not.toBeNull();
        expect(document.body.querySelector(".rad-scrim--modal")).not.toBeNull();

        await fireEvent.click(pane.byText("Cancel")!);
        expect(document.body.querySelector('[aria-label="Gamertag"]')).toBeNull();
    });

    // One field, so the return key is the obvious way to commit it.
    it("adds on Enter", async () => {
        mockInvoke({
            api_introspect: () => ({ permissions: ["admin"] }),
            get_credential: () => JSON.stringify({ allowed: ["admin"] }),
            admin_list_users: () => page([user()]),
            admin_create_user: () => "applied",
        });

        const pane = mount();
        await waitFor(() => expect(pane.text()).toContain("Bob"));

        await fireEvent.click(pane.byText("Add player")!);
        const field = document.body.querySelector<HTMLInputElement>('[aria-label="Gamertag"]')!;
        await fireEvent.input(field, { target: { value: "Carol" } });
        await fireEvent.keyDown(field, { key: "Enter" });

        await waitFor(() => {
            const call = invokeCalls().find((entry) => entry.cmd === "admin_create_user");
            expect((call?.args as { gamertag: string }).gamertag).toBe("Carol");
        });
        // A dialog that succeeded closes itself.
        await waitFor(() =>
            expect(document.body.querySelector('[aria-label="Gamertag"]')).toBeNull(),
        );
    });

    it("says why a duplicate whitelist entry was refused", async () => {
        mockInvoke({
            api_introspect: () => ({ permissions: ["admin"] }),
            get_credential: () => JSON.stringify({ allowed: ["admin"] }),
            admin_list_users: () => page([user()]),
            admin_create_user: () => "conflict",
        });

        const pane = mount();
        await waitFor(() => expect(pane.text()).toContain("Bob"));

        await fireEvent.click(pane.byText("Add player")!);
        const field = document.body.querySelector<HTMLInputElement>('[aria-label="Gamertag"]')!;
        await fireEvent.input(field, { target: { value: "Bob" } });
        await fireEvent.click(pane.byText("Add")!);

        await waitFor(() =>
            expect(pane.text()).toContain("That player is already on the whitelist."),
        );
    });

    // Denying a permission and clearing it are different writes to different routes.
    it("writes an override through the permission editor", async () => {
        mockInvoke({
            api_introspect: () => ({ permissions: ["admin"] }),
            get_credential: () => JSON.stringify({ allowed: ["admin"] }),
            admin_list_users: () => page([user()]),
            admin_list_permissions: () => ({
                gamertag: "Bob",
                game: "minecraft",
                entries: [],
            }),
            admin_set_permission: () => "applied",
        });

        const pane = mount();
        await waitFor(() => expect(pane.text()).toContain("Bob"));

        await fireEvent.click(pane.button("Permissions for Bob")!);
        await waitFor(() => expect(pane.text()).toContain("Administrator"));

        await fireEvent.click(pane.byText("Allow")!);
        await waitFor(() => {
            const call = invokeCalls().find((entry) => entry.cmd === "admin_set_permission");
            expect(call).toBeTruthy();
            expect((call!.args as { effect: string }).effect).toBe("allow");
        });
    });

    // The strip replaced inline role chips. Colour carries it visually and the accessible
    // name carries it for everyone else, so both are asserted.
    it("shows a permission as a coloured block rather than inline text", async () => {
        mockInvoke({
            api_introspect: () => ({ permissions: ["admin"] }),
            get_credential: () => JSON.stringify({ allowed: ["admin"] }),
            admin_list_users: () => page([user({ permissions: ["admin"], connected: true })]),
        });

        const pane = mount();
        await waitFor(() => expect(pane.strips()).toHaveLength(1));

        // The strip opens the editor, so its label names the action and the state it shows.
        expect(pane.strips()[0].getAttribute("aria-label")).toBe(
            "Permissions for Bob \u2014 Online \u00b7 Administrator",
        );
        expect(pane.blocks()).toEqual([
            "var(--color-rad-ok)",
            "var(--color-rad-brand-lift)",
            "var(--color-rad-mortar-dead)",
            "var(--color-rad-mortar-dead)",
        ]);
        // The role is not written into the row any more.
        expect(document.body.querySelector(".rad-table__name")?.textContent?.trim()).toBe("Bob");
    });

    it("paints an offline player grey and a banned one as a fault", async () => {
        mockInvoke({
            api_introspect: () => ({ permissions: ["admin"] }),
            get_credential: () => JSON.stringify({ allowed: ["admin"] }),
            admin_list_users: () =>
                page([user(), user({ gamertag: "Griefer", banished: true })]),
        });

        const pane = mount();
        await waitFor(() => expect(pane.strips()).toHaveLength(2));

        expect(pane.blocks()[0]).toBe("var(--color-rad-line-2)");
        expect(pane.blocks()[4]).toBe("var(--color-rad-fault)");
    });

    it("capitalizes the game", async () => {
        mockInvoke({
            api_introspect: () => ({ permissions: ["admin"] }),
            get_credential: () => JSON.stringify({ allowed: ["admin"] }),
            admin_list_users: () => page([user()]),
        });

        const pane = mount();
        await waitFor(() => expect(pane.text()).toContain("Minecraft"));
        expect(pane.text()).not.toContain("minecraft");
    });

    // Banning yourself is refused by the server, so the button is not offered. The rest of
    // the row stays: an admin may still edit their own non-admin permissions.
    it("offers no ban button on the operator's own row", async () => {
        mockInvoke({
            api_introspect: () => ({ gamertag: "RootAdmin", game: "minecraft", permissions: ["admin"] }),
            get_credential: () => JSON.stringify({ allowed: ["admin"] }),
            admin_list_users: () => page([user({ gamertag: "RootAdmin" }), user()]),
        });

        const pane = mount();
        await waitFor(() => expect(pane.text()).toContain("RootAdmin"));

        await waitFor(() => expect(pane.button("Ban RootAdmin")).toBeNull());
        expect(pane.button("Ban Bob")).not.toBeNull();
        // The way into their own permissions stays: an admin may still grant themselves
        // something that is not admin.
        expect(pane.button("Settings for RootAdmin")).not.toBeNull();
        expect(pane.button("Permissions for RootAdmin")).not.toBeNull();
    });

    // Only Minecraft ships, so the roster is asked for unfiltered. The route still takes
    // the parameter, which is what a second game would use.
    it("asks for the roster unfiltered by game", async () => {
        mockInvoke({
            api_introspect: () => ({ permissions: ["admin"] }),
            get_credential: () => JSON.stringify({ allowed: ["admin"] }),
            admin_list_users: () => page([user()]),
        });

        const pane = mount();
        await waitFor(() => expect(pane.text()).toContain("Bob"));

        const first = invokeCalls().find((call) => call.cmd === "admin_list_users");
        const query = (first!.args as { query: { game: string | null; page_size: number } }).query;
        expect(query.game).toBeNull();
        expect(query.page_size).toBe(8);
    });

    // The strip is the way in, so the row no longer carries a separate gear.
    it("opens the permission editor from the strip", async () => {
        mockInvoke({
            api_introspect: () => ({ permissions: ["admin"] }),
            get_credential: () => JSON.stringify({ allowed: ["admin"] }),
            admin_list_users: () => page([user()]),
            admin_list_permissions: () => ({ gamertag: "Bob", game: "minecraft", entries: [] }),
        });

        const pane = mount();
        await waitFor(() => expect(pane.strips()).toHaveLength(1));

        await fireEvent.click(pane.strips()[0]);
        await waitFor(() => expect(pane.text()).toContain("Administrator"));
        expect(invokeCalls().some((call) => call.cmd === "admin_list_permissions")).toBe(true);
    });

    // `table.css` hides the wide table under 620px and shows `.rad-datacards` instead, so a
    // pane that renders only the table is blank on a phone. Both layouts render, always.
    it("renders a card per row for the narrow layout", async () => {
        mockInvoke({
            api_introspect: () => ({ permissions: ["admin"] }),
            get_credential: () => JSON.stringify({ allowed: ["admin"] }),
            admin_list_users: () =>
                page([user({ connected: true }), user({ gamertag: "Carol" })]),
        });

        const pane = mount();
        await waitFor(() => expect(pane.cards()).toHaveLength(2));

        // Name first, then the state written out — a strip is small on a phone.
        const first = pane.cardText()[0];
        expect(first.startsWith("Bob")).toBe(true);
        expect(first).toContain("Online");
        expect(first).toContain("Minecraft");
    });

    // The same controls, so a phone is not a read-only view of the roster.
    it("carries the strip, the cog and the ban button on a card", async () => {
        mockInvoke({
            api_introspect: () => ({ gamertag: "RootAdmin", game: "minecraft", permissions: ["admin"] }),
            get_credential: () => JSON.stringify({ allowed: ["admin"] }),
            admin_list_users: () => page([user(), user({ gamertag: "RootAdmin" })]),
        });

        const pane = mount();
        await waitFor(() => expect(pane.cards()).toHaveLength(2));

        const card = pane.cards()[0];
        expect(card.querySelector(".rad-matrix__blocks")).not.toBeNull();
        expect(card.querySelector('[aria-label="Settings for Bob"]')).not.toBeNull();
        expect(card.querySelector('[aria-label="Ban Bob"]')).not.toBeNull();

        // And the operator's own card withholds the ban, exactly as their table row does.
        const mine = pane.cards()[1];
        await waitFor(() =>
            expect(mine.querySelector('[aria-label="Ban RootAdmin"]')).toBeNull(),
        );
        expect(mine.querySelector('[aria-label="Settings for RootAdmin"]')).not.toBeNull();
    });

    // Losing the permission mid-session is the one failure that must not read as a network
    // fault: the pane is about to disappear from the sidebar and should say why.
    it("says so when the permission has been taken away", async () => {
        mockInvoke({
            api_introspect: () => ({ permissions: [] }),
            get_credential: () => JSON.stringify({ allowed: [] }),
            admin_list_users: () => {
                throw new Error("Server returned status: 403 Forbidden");
            },
        });

        const pane = mount();
        await waitFor(() =>
            expect(pane.text()).toContain("You no longer hold the admin permission."),
        );
    });
});
