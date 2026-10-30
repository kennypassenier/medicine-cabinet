// Adding stock: a small dialog with a stepper. Quick +/- on the cabinet list
// uses the stepper directly; this dialog is for "I bought a pack".

import { h } from "./dom.js";
import { api } from "./api.js";
import { stepper, wire, stepperValue } from "./controls.js";
import { qty } from "./format.js";
import { reportError, say } from "./ui.js";

/**
 * @param {any} med
 * @param {() => void} onChange called after the stock changed
 */
export function addStock(med, onChange) {
  const dialog = /** @type {HTMLDialogElement} */ (h("dialog", { class: "kp-dialog mc-dialog", "aria-labelledby": "stock-t", "aria-describedby": "stock-d" }));
  const field = stepper({ label: "Amount to add", value: 1, min: 1, step: 1, unit: med.stock_unit });
  const submit = h("button", { type: "submit", class: "kp-button kp-button--primary" }, "Add stock");
  const form = h("form", { method: "dialog", class: "kp-form" },
    h("div", { class: "mc-dialog-field" }, h("span", { class: "kp-field__label" }, "Amount to add"), field),
    h("div", { class: "kp-dialog__actions" },
      h("button", { type: "button", class: "kp-button", onclick: () => dialog.close() }, "Cancel"), submit));
  dialog.append(
    h("h2", { class: "kp-dialog__title", id: "stock-t" }, `Add stock: ${med.name}`),
    h("p", { class: "kp-dialog__description", id: "stock-d" }, `Now ${qty(med.stock, med.stock_unit)} in the cabinet.`),
    form);
  form.addEventListener("submit", async (e) => {
    e.preventDefault();
    const delta = stepperValue(field);
    submit.disabled = true;
    try {
      const updated = await api.addStock(med.id, delta);
      dialog.close();
      say(`Added ${qty(delta, med.stock_unit)}. ${qty(updated.stock, updated.stock_unit)} now.`);
      onChange();
    } catch (err) {
      submit.disabled = false;
      reportError(`Could not add stock to ${med.name}.`, err);
    }
  });
  dialog.addEventListener("close", () => dialog.remove());
  document.body.append(dialog);
  wire(dialog);
  dialog.showModal();
  field.querySelector("input")?.focus();
}
