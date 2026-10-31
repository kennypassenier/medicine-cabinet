// Adding stock: the package's confirmation with a stepper in it. Quick +/- on
// the cabinet list uses the stepper directly; this dialog is for "I bought a
// pack". The accept button says what will happen ("Add 3 tablets") and goes
// off while the amount makes no sense.

import { openConfirmation, attachAcceptFollows } from "/static/kp/dist/kp-themes.min.js";
import { h } from "./dom.js";
import { api } from "./api.js";
import { stepper, wire } from "./controls.js";
import { qty } from "./format.js";
import { reportError, say } from "./ui.js";

/**
 * @param {any} med
 * @param {() => void} onChange called after the stock changed
 */
export function addStock(med, onChange) {
  let amount = 1;
  const id = `stock-qty-${med.id}`;
  const field = stepper({ id, label: "Amount to add", value: amount, min: 1, step: 1, unit: med.stock_unit, onChange: (v) => { amount = v; } });
  const body = h("div", { class: "kp-stack kp-gap-sm" },
    h("p", null, `Now ${qty(med.stock, med.stock_unit)} in the cabinet.`),
    h("div", { class: "kp-field" }, h("span", { class: "kp-field__label" }, "Amount to add"), field));
  const opener = document.activeElement instanceof HTMLElement ? document.activeElement : document.body;
  const dialog = openConfirmation(opener, {
    phrase: `Add stock: ${med.name}`,
    description: body,
    acceptLook: "primary",
    acceptLabel: "Add",
    acceptFollows: id,
    acceptVerb: "Add",
    onAccept: () => void save(med, amount, onChange),
  });
  wire(dialog);
  attachAcceptFollows(dialog);
  field.querySelector("input")?.focus();
}

/** @param {any} med @param {number} delta @param {() => void} onChange */
async function save(med, delta, onChange) {
  try {
    const updated = await api.addStock(med.id, delta);
    say(`Added ${qty(delta, med.stock_unit)}. ${qty(updated.stock, updated.stock_unit)} now.`);
    onChange();
  } catch (err) {
    reportError(`Could not add stock to ${med.name}.`, err);
  }
}
