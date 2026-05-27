import { describe, expect, it } from "vitest";
import {
  backgroundPermissionMessage,
  shouldAskForBackgroundPermission,
  withBackgroundPermission,
  parseAutoChangeMinutes,
  parseCacheLimitMb,
} from "./settingsFlow";
import { defaultSettings } from "./types";

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

describe("background permission helpers", () => {
  it("asks when auto-change needs background and startup permission", () => {
    const next = { ...defaultSettings, autoChangeMinutes: 15 };

    expect(shouldAskForBackgroundPermission(defaultSettings, next)).toBe(true);
  });

  it("does not ask again after background and startup are enabled", () => {
    const current = {
      ...defaultSettings,
      autoChangeMinutes: 15,
      runInBackground: true,
      launchAtStartup: true,
    };

    expect(shouldAskForBackgroundPermission(current, current)).toBe(false);
  });

  it("enables both background runtime and startup after permission", () => {
    expect(withBackgroundPermission(defaultSettings)).toMatchObject({
      runInBackground: true,
      launchAtStartup: true,
    });
  });

  it("uses direct language for the OS permission prompt", () => {
    expect(backgroundPermissionMessage).toContain("run in the tray");
    expect(backgroundPermissionMessage).toContain("start at login");
  });
});
