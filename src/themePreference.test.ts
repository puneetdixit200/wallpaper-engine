import { describe, expect, it } from "vitest";
import { resolveThemePreference } from "./themePreference";

describe("resolveThemePreference", () => {
  it("resolves system theme without duplicating CSS theme variables", () => {
    expect(resolveThemePreference("system", true)).toBe("dark");
    expect(resolveThemePreference("system", false)).toBe("light");
    expect(resolveThemePreference("dark", false)).toBe("dark");
    expect(resolveThemePreference("light", true)).toBe("light");
  });
});
