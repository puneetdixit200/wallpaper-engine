import { describe, expect, it } from "vitest";
import {
  hotkeyCaptureFromKeyboardEvent,
  hotkeyFromKeyboardEvent,
  HotkeyKeyboardEvent,
} from "./hotkeyCapture";

function event(
  next: Partial<HotkeyKeyboardEvent>,
): HotkeyKeyboardEvent {
  return {
    altKey: false,
    code: "KeyN",
    ctrlKey: false,
    key: "n",
    metaKey: false,
    shiftKey: false,
    ...next,
  };
}

describe("hotkeyFromKeyboardEvent", () => {
  it("records modifier and letter combinations", () => {
    expect(
      hotkeyFromKeyboardEvent(
        event({ altKey: true, code: "KeyN", metaKey: true }),
      ),
    ).toBe("Command+Alt+N");
  });

  it("records arrows and function keys", () => {
    expect(
      hotkeyFromKeyboardEvent(
        event({ code: "ArrowRight", ctrlKey: true, key: "ArrowRight" }),
      ),
    ).toBe("Control+ArrowRight");
    expect(
      hotkeyFromKeyboardEvent(
        event({ code: "F8", ctrlKey: true, key: "F8", shiftKey: true }),
      ),
    ).toBe("Control+Shift+F8");
  });

  it("ignores modifier-only keydown events", () => {
    expect(hotkeyFromKeyboardEvent(event({ code: "MetaLeft", key: "Meta" }))).toBeNull();
  });

  it("shows partial modifier captures before the final key is pressed", () => {
    expect(
      hotkeyCaptureFromKeyboardEvent(
        event({ code: "ControlLeft", ctrlKey: true, key: "Control" }),
      ),
    ).toEqual({
      displayString: "Control",
      parts: ["Control"],
      value: null,
    });
    expect(
      hotkeyCaptureFromKeyboardEvent(
        event({
          code: "ShiftLeft",
          ctrlKey: true,
          key: "Shift",
          shiftKey: true,
        }),
      ),
    ).toMatchObject({
      displayString: "Control + Shift",
      value: null,
    });
  });
});
