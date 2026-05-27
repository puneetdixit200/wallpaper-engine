import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";

const css = readFileSync(new URL("./App.css", import.meta.url), "utf8");

describe("desktop sidebar layout", () => {
  it("keeps the left panel fixed while content scrolls", () => {
    const sidebarRule = css.match(/\.sidebar\s*\{(?<body>[^}]+)\}/)?.groups
      ?.body;

    expect(sidebarRule).toContain("position: sticky");
    expect(sidebarRule).toContain("top: 0");
    expect(sidebarRule).toContain("height: 100vh");
    expect(sidebarRule).toContain("align-self: start");
  });
});
