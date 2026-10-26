import { beforeEach, describe, expect, it } from "vitest";
import { get } from "svelte/store";
import { mockInvoke } from "../../tauri";
import Login from "../../../js/app/login";

/**
 * A failed check puts its reason under the field, on the editable form, so the player can
 * correct what they typed and submit again.
 */
describe("Login with a server that cannot be reached", () => {
    beforeEach(() => mockInvoke({}));

    it("shows the classified reason inline and returns to the form", async () => {
        mockInvoke({
            check_server: () => ({
                state: "Failed",
                failure: { kind: "WrongCertificateName", presented: ["panel.restcraft.net"] },
                detail: "certificate not valid for name",
            }),
        });
        const login = new Login();
        login.setServerInput("restcraft.net");

        await login.runLogin("ms");

        expect(get(login.formError)).toBe(
            "That address belongs to panel.restcraft.net. It may be your game server or panel, not the voice server",
        );
        expect(get(login.serverInputInvalid)).toBe(true);
        expect(get(login.loginState)).toBe("idle");
    });
});
