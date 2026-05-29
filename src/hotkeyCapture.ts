const ignoredKeys = new Set([
  "OS",
  "Fn",
  "FnLock",
]);

const modifierKeys = new Set(["Alt", "Control", "Meta", "Shift"]);

const codeLabels: Record<string, string> = {
  Backquote: "Backquote",
  Backslash: "Backslash",
  BracketLeft: "BracketLeft",
  BracketRight: "BracketRight",
  Comma: "Comma",
  Equal: "Equal",
  Minus: "Minus",
  Period: "Period",
  Quote: "Quote",
  Semicolon: "Semicolon",
  Slash: "Slash",
  Space: "Space",
};

export interface HotkeyKeyboardEvent {
  altKey: boolean;
  code: string;
  ctrlKey: boolean;
  key: string;
  metaKey: boolean;
  shiftKey: boolean;
}

export interface HotkeyCaptureSnapshot {
  displayString: string;
  parts: string[];
  value: string | null;
}

export function hotkeyCaptureFromKeyboardEvent(
  event: HotkeyKeyboardEvent,
): HotkeyCaptureSnapshot | null {
  if (ignoredKeys.has(event.key)) {
    return null;
  }

  const modifiers = modifierParts(event);
  const key = modifierKeys.has(event.key)
    ? null
    : hotkeyKeyFromCode(event.code, event.key);
  const parts = key ? [...modifiers, key] : modifiers;

  if (parts.length === 0) {
    return null;
  }

  return {
    displayString: parts.join(" + "),
    parts,
    value: key ? parts.join("+") : null,
  };
}

export function hotkeyFromKeyboardEvent(
  event: HotkeyKeyboardEvent,
): string | null {
  return hotkeyCaptureFromKeyboardEvent(event)?.value ?? null;
}

function modifierParts(event: HotkeyKeyboardEvent): string[] {
  const modifiers: string[] = [];
  if (event.metaKey || event.key === "Meta") {
    modifiers.push("Command");
  }
  if (event.ctrlKey || event.key === "Control") {
    modifiers.push("Control");
  }
  if (event.altKey || event.key === "Alt") {
    modifiers.push("Alt");
  }
  if (event.shiftKey || event.key === "Shift") {
    modifiers.push("Shift");
  }
  return modifiers;
}

function hotkeyKeyFromCode(code: string, key: string): string | null {
  if (/^Key[A-Z]$/.test(code)) {
    return code.slice(3);
  }

  if (/^Digit[0-9]$/.test(code)) {
    return code.slice(5);
  }

  if (/^Numpad[0-9]$/.test(code)) {
    return code;
  }

  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) {
    return code;
  }

  if (
    [
      "ArrowDown",
      "ArrowLeft",
      "ArrowRight",
      "ArrowUp",
      "Backspace",
      "CapsLock",
      "Delete",
      "End",
      "Escape",
      "Home",
      "Insert",
      "PageDown",
      "PageUp",
      "Pause",
      "PrintScreen",
      "ScrollLock",
      "Tab",
    ].includes(code)
  ) {
    return code;
  }

  return codeLabels[code] ?? printableKey(key);
}

function printableKey(key: string): string | null {
  return key.length === 1 ? key.toUpperCase() : null;
}
