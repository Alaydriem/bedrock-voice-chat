import { describe, expect, it } from "vitest";
import { AddServerRoute } from "../../../js/app/server/AddServerRoute";

function paramsOf(href: string): URLSearchParams {
  return new URLSearchParams(href.slice(href.indexOf("?")));
}

describe("AddServerRoute", () => {
  // The defect this guards. Adding a server used to lead to the roster, and a device with one
  // saved server is forwarded off the roster to that server's dashboard before the roster
  // draws — so the button returned the user to the screen they pressed it on.
  it("leads to a sign-in that knows a server is being added", () => {
    expect(AddServerRoute.HREF.startsWith("/login")).toBe(true);
    expect(paramsOf(AddServerRoute.HREF).has("addserver")).toBe(true);
  });

  it("offers a way out of the sign-in it sends people to", () => {
    expect(AddServerRoute.backFrom(paramsOf(AddServerRoute.HREF)).href).toBe(
      AddServerRoute.RETURN_TO,
    );
  });

  it("names the way out without promising a screen that may be passed through", () => {
    expect(AddServerRoute.backFrom(paramsOf(AddServerRoute.HREF)).label).toBe("Cancel");
  });

  it("sends a sign-in that is not adding a server back to the dashboard", () => {
    expect(AddServerRoute.backFrom(new URLSearchParams("reauth=true&server=x")).href).toBe(
      "/dashboard",
    );
  });

  // A return target is a destination this app navigates to unprompted, so only the one it
  // issues itself is honoured.
  it("ignores a return target it did not issue", () => {
    expect(
      AddServerRoute.backFrom(new URLSearchParams("addserver=true&return=https://evil.test"))
        .href,
    ).toBe("/dashboard");
  });
});
