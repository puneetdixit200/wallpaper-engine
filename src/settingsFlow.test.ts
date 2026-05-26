import { describe, expect, it } from "vitest";
import { parseAutoChangeMinutes, parseCacheLimitMb } from "./settingsFlow";

describe("parseAutoChangeMinutes", () => {
  it("keeps custom minute values and clamps invalid input", () => {
    expect(parseAutoChangeMinutes("7")).toBe(7);
    expect(parseAutoChangeMinutes("0")).toBe(0);
    expect(parseAutoChangeMinutes("20000")).toBe(1440);
    expect(parseAutoChangeMinutes("-5")).toBe(0);
    expect(parseAutoChangeMinutes("")).toBe(0);
  });
});

describe("parseCacheLimitMb", () => {
  it("keeps cache limits inside the supported backend range", () => {
    expect(parseCacheLimitMb("512")).toBe(512);
    expect(parseCacheLimitMb("127")).toBe(128);
    expect(parseCacheLimitMb("20000")).toBe(10240);
    expect(parseCacheLimitMb("abc")).toBe(1024);
  });
});
