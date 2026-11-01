// A hidden text field that stands for state the unsaved-changes bar cannot see
// (chips, the schedule's rows). The bar tracks it like any field: a change in
// the state calls `sync()`, and Discard / Undo / Revert write the field, which
// calls `write` so the state follows. The bar calls the field's value "was/now".

import { h } from "../dom.js";

/**
 * @param {{ kind: string, read: () => string, write: (value: string) => void }} o
 *   `read` serialises the state; `write` puts a serialised state back (must not call `sync`)
 */
export function mirror({ kind, read, write }) {
  const input = /** @type {HTMLInputElement} */ (h("input", { type: "text", hidden: true, tabindex: "-1", "aria-hidden": "true", "data-kp-mirror": kind }));
  input.value = read();
  let writing = false;
  input.addEventListener("change", () => {
    if (writing || input.value === read()) return;
    writing = true;
    try {
      write(input.value);
    } finally {
      writing = false;
    }
  });
  return {
    input,
    /** The state changed by hand: tell the bar. */
    sync() {
      const v = read();
      if (writing || input.value === v) return;
      input.value = v;
      input.dispatchEvent(new Event("input", { bubbles: true }));
      input.dispatchEvent(new Event("change", { bubbles: true }));
    },
  };
}
