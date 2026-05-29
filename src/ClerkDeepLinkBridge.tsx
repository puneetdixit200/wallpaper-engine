import { useEffect, useRef } from "react";
import { useClerk } from "@clerk/clerk-react";
import { getCurrent, onOpenUrl } from "@tauri-apps/plugin-deep-link";
import {
  clerkErrorMessage,
  desktopAuthCallbackUrlToLocalPath,
  emitClerkAuthEvent,
  isDesktopAuthCallbackUrl,
} from "./clerkDesktopAuth";
import { logAppAction } from "./appLog";

interface ClerkDeepLinkBridgeProps {
  onAuthenticated: () => void;
}

const isTauriRuntime = () =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export function ClerkDeepLinkBridge({
  onAuthenticated,
}: ClerkDeepLinkBridgeProps) {
  const clerk = useClerk();
  const processedUrls = useRef(new Set<string>());

  useEffect(() => {
    if (!isTauriRuntime() || !clerk.loaded) {
      return;
    }

    let cancelled = false;
    let unlisten: (() => void) | undefined;

    async function processUrls(urls: string[] | null | undefined) {
      if (!urls || cancelled) {
        return;
      }
      for (const url of urls) {
        if (
          cancelled ||
          processedUrls.current.has(url) ||
          !isDesktopAuthCallbackUrl(url)
        ) {
          continue;
        }
        processedUrls.current.add(url);
        void logAppAction("clerk.deep_link.received", "Clerk deep link received.");
        await completeClerkRedirect(url);
      }
    }

    async function completeClerkRedirect(url: string) {
      emitClerkAuthEvent({
        state: "checking",
        message: "Finishing browser sign-in...",
      });
      try {
        void logAppAction(
          "clerk.deep_link.complete.start",
          "Completing Clerk browser sign-in callback.",
        );
        const callbackPath = desktopAuthCallbackUrlToLocalPath(url);
        window.history.replaceState({}, "", callbackPath);
        await clerk.handleRedirectCallback(
          {
            signInForceRedirectUrl: "/",
            signUpForceRedirectUrl: "/",
            signInFallbackRedirectUrl: "/",
            signUpFallbackRedirectUrl: "/",
          },
          async () => undefined,
        );
        window.history.replaceState({}, "", "/");
        onAuthenticated();
        emitClerkAuthEvent({
          state: "success",
          message: "Signed in with Clerk.",
        });
        void logAppAction(
          "clerk.deep_link.complete.success",
          "Clerk browser sign-in completed.",
        );
      } catch (error) {
        window.history.replaceState({}, "", "/");
        emitClerkAuthEvent({
          state: "error",
          message: `Clerk browser sign-in failed: ${clerkErrorMessage(error)}`,
        });
        void logAppAction(
          "clerk.deep_link.complete.error",
          "Clerk browser sign-in callback failed.",
          { error: clerkErrorMessage(error) },
          "error",
        );
      }
    }

    void getCurrent()
      .then(processUrls)
      .catch((error) =>
        {
          emitClerkAuthEvent({
            state: "error",
            message: `Could not read deep link callback: ${clerkErrorMessage(error)}`,
          });
          void logAppAction(
            "clerk.deep_link.current.error",
            "Could not read current deep link callback.",
            { error: clerkErrorMessage(error) },
            "error",
          );
        },
      );

    void onOpenUrl((urls) => {
      void processUrls(urls);
    })
      .then((nextUnlisten) => {
        unlisten = nextUnlisten;
      })
      .catch((error) =>
        {
          emitClerkAuthEvent({
            state: "error",
            message: `Could not listen for deep links: ${clerkErrorMessage(error)}`,
          });
          void logAppAction(
            "clerk.deep_link.listen.error",
            "Could not listen for Clerk deep links.",
            { error: clerkErrorMessage(error) },
            "error",
          );
        },
      );

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [clerk, onAuthenticated]);

  return null;
}
