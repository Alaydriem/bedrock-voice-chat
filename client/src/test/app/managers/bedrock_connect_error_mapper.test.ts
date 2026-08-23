import { describe, expect, it } from "vitest";
import { BedrockConnectErrorMapper } from "../../../js/app/managers/bedrock/connection/BedrockConnectErrorMapper";

describe("BedrockConnectErrorMapper", () => {
    it("tells the player to sign in again when the credential was rejected", () => {
        const described = BedrockConnectErrorMapper.describe({ kind: "reauth_required" });

        expect(described.title).toMatch(/sign in/i);
        // Every other arm tells the player to click Refresh to renew tokens. By the time this
        // arm is reached the app has already tried exactly that on their behalf, so repeating
        // the advice sends them round a loop that cannot help.
        expect(described.suggestion).not.toMatch(/refresh/i);
    });

    it("still tells the player to refresh on an ordinary auth failure", () => {
        const described = BedrockConnectErrorMapper.describe({
            kind: "auth",
            message: "token refresh failed (503): upstream",
        });

        expect(described.suggestion).toMatch(/refresh/i);
    });
});
