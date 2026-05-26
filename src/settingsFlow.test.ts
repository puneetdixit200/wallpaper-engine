import { describe, expect, it } from "vitest";
import { parseAutoChangeMinutes } from "./settingsFlow";

describe("parseAutoChangeMinutes", () => {
  it("keeps custom minute values and clamps invalid input", () => {
    expect(parseAutoChangeMinutes("7")).toBe(7);
    expect(parseAutoChangeMinutes("0")).toBe(0);
    expect(parseAutoChangeMinutes("20000")).toBe(1440);
    expect(parseAutoChangeMinutes("-5")).toBe(0);
    expect(parseAutoChangeMinutes("")).toBe(0);
  });
});
