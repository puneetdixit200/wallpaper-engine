import { describe, expect, it, vi } from "vitest";
import { resetDocumentScroll } from "./viewScroll";

describe("view scroll handling", () => {
  it("returns the document to the top when switching views", () => {
    const scrollTo = vi.fn();

    resetDocumentScroll({ scrollTo });

    expect(scrollTo).toHaveBeenCalledWith({
      top: 0,
      left: 0,
      behavior: "auto",
    });
  });
});
