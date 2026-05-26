import { describe, expect, it } from "vitest";
import { sourceSelectionSearch } from "./searchFlow";

describe("sourceSelectionSearch", () => {
  it("starts a fresh search with the newly selected source", () => {
    expect(sourceSelectionSearch("anime city", "wallhaven")).toEqual({
      nextPage: 1,
      nextQuery: "anime city",
      nextSource: "wallhaven",
    });
  });
});
