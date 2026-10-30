// A question dialog built from the package's `.kp-dialog` parts. The native
// <dialog> traps focus, closes on Escape and returns focus to the opener;
// focus starts on Cancel so a stray Enter never confirms something harmful.

import { h } from "./dom.js";

/**
 * Ask a yes/no question.
 * @param {{ title: string, text: string | Node, accept: string, destructive?: boolean, cancel?: string }} q
 * @returns {Promise<boolean>} true when accepted
 */
export function ask({ title, text, accept, destructive = false, cancel = "Cancel" }) {
  return new Promise((resolve) => {
    const uid = `dlg-${Math.random().toString(36).slice(2, 8)}`;
    const dialog = /** @type {HTMLDialogElement} */ (h("dialog", { class: "kp-dialog kp-confirm mc-dialog", "aria-labelledby": `${uid}-t`, "aria-describedby": `${uid}-d` }));
    let answer = false;
    const close = (/** @type {boolean} */ yes) => {
      answer = yes;
      dialog.close();
    };
    const no = h("button", { type: "button", class: "kp-button", onclick: () => close(false) }, cancel);
    const yes = h("button", { type: "button", class: `kp-button ${destructive ? "kp-button--destructive" : "kp-button--primary"}`, onclick: () => close(true) }, accept);
    dialog.append(
      h("h2", { class: "kp-dialog__title", id: `${uid}-t` }, title),
      h("div", { class: "kp-dialog__description", id: `${uid}-d` }, text),
      h("div", { class: "kp-dialog__actions" }, no, yes));
    dialog.addEventListener("close", () => {
      dialog.remove();
      resolve(answer);
    });
    document.body.append(dialog);
    dialog.showModal();
    no.focus();
  });
}
