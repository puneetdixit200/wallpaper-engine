import { invoke } from "@tauri-apps/api/core";

type AppLogLevel = "debug" | "info" | "warn" | "error";

const isTauriRuntime = () =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export async function logAppAction(
  action: string,
  message: string,
  details: Record<string, unknown> = {},
  level: AppLogLevel = "info",
) {
  if (!isTauriRuntime()) {
    return;
  }

  try {
    await invoke("write_app_log", {
      entry: {
        level,
        action,
        message,
        details,
      },
    });
  } catch (error) {
    console.warn("Could not write app log", error);
  }
}
