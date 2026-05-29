import { describe, expect, it } from "vitest";
import {
  desktopAuthBridgeUrl,
  desktopAuthCallbackUrl,
  desktopAuthCallbackUrlToLocalPath,
  externalClerkVerificationUrl,
  isDesktopAuthCallbackUrl,
} from "./clerkDesktopAuth";

describe("clerk desktop auth helpers", () => {
  it("accepts the configured desktop auth callback URL", () => {
    expect(isDesktopAuthCallbackUrl(desktopAuthCallbackUrl)).toBe(true);
    expect(
      desktopAuthCallbackUrlToLocalPath(
        `${desktopAuthCallbackUrl}?__clerk_status=complete#token`,
      ),
    ).toBe("/auth/callback?__clerk_status=complete#token");
  });

  it("rejects unrelated deep links", () => {
    expect(isDesktopAuthCallbackUrl("wallpaper-engine://library/open")).toBe(
      false,
    );
    expect(isDesktopAuthCallbackUrl("https://example.com/auth/callback")).toBe(
      false,
    );
  });

  it("uses an HTTPS bridge URL for Clerk OAuth", () => {
    expect(new URL(desktopAuthBridgeUrl).protocol).toBe("https:");
    expect(desktopAuthBridgeUrl).toContain("/auth/callback/");
  });

  it("extracts the Clerk OAuth verification URL", () => {
    expect(
      externalClerkVerificationUrl({
        firstFactorVerification: {
          externalVerificationRedirectURL: new URL(
            "https://accounts.example.dev/oauth",
          ),
        },
      }),
    ).toBe("https://accounts.example.dev/oauth");
  });

  it("fails clearly when Clerk does not return an OAuth URL", () => {
    expect(() =>
      externalClerkVerificationUrl({
        firstFactorVerification: {
          externalVerificationRedirectURL: null,
        },
      }),
    ).toThrow("Google OAuth is enabled");
  });
});
