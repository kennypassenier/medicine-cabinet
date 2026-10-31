// A question dialog: the package's confirmation (`openConfirmation`). The native
// <dialog> traps focus, closes on Escape and returns focus to the opener; focus
// starts on Cancel so a stray Enter never confirms something harmful.

import { openConfirmation } from "/static/kp/dist/kp-themes.min.js";

/**
 * Ask a yes/no question. `text` may be a node (several paragraphs).
 * @param {{ title: string, text: string | Node, accept: string, destructive?: boolean, cancel?: string, opener?: HTMLElement }} q
 * @returns {Promise<boolean>} true when accepted
 */
export function ask({ title, text, accept, destructive = false, cancel = "Cancel", opener }) {
  const from = opener ?? (document.activeElement instanceof HTMLElement ? document.activeElement : document.body);
  return new Promise((resolve) => {
    openConfirmation(from, {
      phrase: title,
      description: text,
      acceptLabel: accept,
      acceptLook: destructive ? "destructive" : "primary",
      strings: { confirmCancel: cancel },
      onAccept: () => resolve(true),
      onCancel: () => resolve(false),
    });
  });
}
