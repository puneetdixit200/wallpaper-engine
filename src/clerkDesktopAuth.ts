export const desktopAuthScheme = "wallpaper-engine";
export const desktopAuthCallbackUrl = "wallpaper-engine://auth/callback";

export type ClerkAuthEventState = "success" | "error" | "checking";

export interface ClerkAuthEventDetail {
  state: ClerkAuthEventState;
  message: string;
}

interface ClerkOAuthAttempt {
  firstFactorVerification: {
    externalVerificationRedirectURL: URL | string | null;
  };
}

export function isDesktopAuthCallbackUrl(value: string): boolean {
  const url = parseUrl(value);
  if (!url || url.protocol !== `${desktopAuthScheme}:`) {
    return false;
  }
  return (
    (url.hostname === "auth" && url.pathname === "/callback") ||
    (url.hostname === "" && url.pathname === "/auth/callback")
  );
}

export function desktopAuthCallbackUrlToLocalPath(value: string): string {
  const url = parseUrl(value);
  if (!url || !isDesktopAuthCallbackUrl(value)) {
    throw new Error("Invalid Clerk desktop callback URL.");
  }
  return `/auth/callback${url.search}${url.hash}`;
}

export function emitClerkAuthEvent(detail: ClerkAuthEventDetail) {
  window.dispatchEvent(
    new CustomEvent<ClerkAuthEventDetail>("wallpaper-engine-clerk-auth", {
      detail,
    }),
  );
}

export function externalClerkVerificationUrl(
  attempt: ClerkOAuthAttempt,
): string {
  const redirectUrl =
    attempt.firstFactorVerification.externalVerificationRedirectURL;
  if (!redirectUrl) {
    throw new Error(
      "Clerk did not return a browser verification URL. Check that Google OAuth is enabled for this Clerk app.",
    );
  }
  return redirectUrl.toString();
}

export function clerkErrorMessage(error: unknown): string {
  if (error instanceof Error && error.message.trim()) {
    return error.message;
  }
  return String(error);
}

function parseUrl(value: string): URL | null {
  try {
    return new URL(value);
  } catch {
    return null;
  }
}
