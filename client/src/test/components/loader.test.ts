import { describe, expect, it } from "vitest";
import { render, screen, waitFor } from "@testing-library/svelte";
import Loader from "$radial/components/Loader.svelte";

describe("Loader", () => {
  it("says nothing at all when it has no phrases", async () => {
    render(Loader, { props: { loading: true } });
    await new Promise((r) => setTimeout(r, 30));
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  it("stays silent while the wait is still short", async () => {
    render(Loader, {
      props: { loading: true, phrases: ["Reaching your server…"], slowAfterSeconds: 10 },
    });
    await new Promise((r) => setTimeout(r, 40));
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  // A threshold of zero takes the same code path a four-second wait takes,
  // without making the test wait four seconds.
  it("explains itself once the wait is long enough", async () => {
    render(Loader, {
      props: { loading: true, phrases: ["Reaching your server…"], slowAfterSeconds: 0 },
    });
    await waitFor(() =>
      expect(screen.getByRole("status")).toHaveTextContent("Reaching your server…"),
    );
  });

  it("announces the status politely rather than interrupting", async () => {
    render(Loader, {
      props: { loading: true, phrases: ["Reaching your server…"], slowAfterSeconds: 0 },
    });
    await waitFor(() => expect(screen.getByRole("status")).toHaveAttribute("aria-live", "polite"));
  });

  // The glyph is decoration beside text that already says the same thing, so it
  // must not be read out twice.
  it("hides the spinner glyph from assistive technology", async () => {
    const { container } = render(Loader, {
      props: { loading: true, phrases: ["Reaching your server…"], slowAfterSeconds: 0 },
    });
    await waitFor(() => expect(screen.getByRole("status")).toBeInTheDocument());
    expect(container.querySelector(".rad-loader__glyph")).toHaveAttribute("aria-hidden", "true");
  });

  it("stops explaining once loading is over", async () => {
    const { rerender } = render(Loader, {
      props: { loading: true, phrases: ["Reaching your server…"], slowAfterSeconds: 0 },
    });
    await waitFor(() => expect(screen.getByRole("status")).toBeInTheDocument());
    await rerender({ loading: false, phrases: ["Reaching your server…"], slowAfterSeconds: 0 });
    await waitFor(() => expect(screen.queryByRole("status")).not.toBeInTheDocument());
  });
});
