// Reversible actions: do it at once, then say what happened with an Undo toast
// (the package's `undoToast`, mode revert, 8 s) instead of asking first.

import { undoToast } from "/static/kp/dist/kp-themes.min.js";
import { reportError } from "./ui.js";

/** How long Undo is offered, in milliseconds. */
export const UNDO_MS = 8000;

/**
 * @param {string} text what was done, as a sentence
 * @param {{ undo: () => Promise<unknown>, failed: string, after?: () => void }} o
 *   `undo` is the reverse call; `failed` names what could not be undone; `after` runs once the undo went through (redraw the screen)
 */
export function undoable(text, { undo, failed, after }) {
  undoToast(text, {
    mode: "revert",
    ms: UNDO_MS,
    undo: async () => {
      try {
        await undo();
        after?.();
      } catch (e) {
        reportError(failed, e);
      }
    },
  });
}
