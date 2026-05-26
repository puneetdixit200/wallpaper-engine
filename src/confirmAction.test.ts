import { describe, expect, it, vi } from "vitest";
import { runConfirmed } from "./confirmAction";

describe("runConfirmed", () => {
  it("skips destructive actions when the user cancels", async () => {
    const action = vi.fn();

    const ran = await runConfirmed(() => false, "Clear everything?", action);

    expect(ran).toBe(false);
    expect(action).not.toHaveBeenCalled();
  });

  it("runs destructive actions after confirmation", async () => {
    const action = vi.fn();

    const ran = await runConfirmed(() => true, "Clear everything?", action);

    expect(ran).toBe(true);
    expect(action).toHaveBeenCalledOnce();
  });
});
