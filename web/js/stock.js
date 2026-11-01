// Adding stock: a popover hanging under the "Add stock" button, holding the
// quantity stepper and an accept button whose words follow the number ("Add 3
// tablets"). Accepting saves at once and offers an Undo toast that reverts
// with a negative delta. No modal: the page behind stays in view.

import { attachAcceptFollows } from "/static/kp/dist/kp-themes.min.js";
import { h } from "./dom.js";
import { api } from "./api.js";
import { stepper, stepperValue, wire } from "./controls.js";
import { qty } from "./format.js";
import { reportError } from "./ui.js";
import { undoable } from "./undo.js";

/** Remove the popover from the page once it has closed. @param {HTMLElement} pop @param {HTMLElement} opener */
function cleanUpOnClose(pop, opener) {
  pop.addEventListener("toggle", (e) => {
    if (/** @type {ToggleEvent} */ (e).newState !== "closed") return;
    pop.remove();
    opener.style.removeProperty("anchor-name");
    if (opener.isConnected) opener.focus({ preventScroll: true });
  });
}

/** The popover's content: what is in the cabinet now, the stepper, the accept button. @param {any} med @param {string} qid @param {(amount: number) => void} accept */
function popoverBody(med, qid, accept) {
  const field = stepper({ id: qid, label: "Amount to add", value: 1, min: 1, step: 1, unit: med.stock_unit });
  const go = h("button", { type: "button", class: "kp-button kp-button--primary kp-w-full", "data-kp-accept-follows": qid, "data-kp-accept-verb": "Add", onclick: () => accept(stepperValue(field)) }, "Add");
  return h("div", { class: "kp-stack kp-gap-sm kp-p-md" },
    h("p", { class: "kp-m-0" }, h("strong", null, med.name), ` · now ${qty(med.stock, med.stock_unit)}`),
    h("div", { class: "kp-field" }, h("span", { class: "kp-field__label" }, "Amount to add"), field),
    go);
}

/**
 * Open the popover under `opener`.
 * @param {any} med @param {HTMLElement} opener the "Add stock" button @param {() => void} onChange called after the stock changed
 */
export function addStock(med, opener, onChange) {
  const anchor = `--stock-${med.id}`;
  const pop = h("div", { popover: "auto", class: "kp-popover", id: `stock-pop-${med.id}`, role: "dialog", "aria-label": `Add stock to ${med.name}` });
  pop.style.setProperty("position-anchor", anchor);
  // Hangs under the button towards its start (the band's buttons stand at the right edge) and flips when there is no room.
  pop.style.setProperty("position-area", "block-end span-inline-start");
  pop.style.setProperty("position-try-fallbacks", "flip-inline");
  opener.style.setProperty("anchor-name", anchor);
  pop.append(popoverBody(med, `stock-qty-${med.id}`, (amount) => {
    pop.hidePopover();
    void save(med, amount, onChange);
  }));
  document.body.append(pop);
  cleanUpOnClose(pop, opener);
  wire(pop);
  attachAcceptFollows(pop);
  pop.showPopover();
  pop.querySelector("input")?.focus();
}

/** @param {any} med @param {number} amount @param {() => void} onChange */
async function save(med, amount, onChange) {
  try {
    await api.addStock(med.id, amount);
  } catch (err) {
    return reportError(`Could not add stock to ${med.name}.`, err);
  }
  onChange();
  undoable(`Added ${qty(amount, med.stock_unit)} to ${med.name}.`, { undo: () => api.addStock(med.id, -amount), failed: `Could not take the stock of ${med.name} back.`, after: onChange });
}
