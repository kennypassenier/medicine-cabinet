// Builders for the package's form controls, so a screen writes one line and
// every stepper / switch / chip row is the same markup everywhere.

import { attachQuantities, QUANTITY_EVENT, getQuantity } from "/static/kp/dist/kp-themes.min.js";
import { h } from "./dom.js";

/**
 * A quantity stepper (minus, number, plus). Call `wire(container)` once it is in the page.
 * The number track fills its cell (`data-kp-quantity-fill`) so every stepper of a column ends on one edge.
 * @param {{ label: string, value: number, min?: number, max?: number, step?: number, unit?: string, onChange?: (v: number) => void, key?: string, id?: string }} o
 */
export function stepper({ label, value, min = 0, max, step = 1, unit = "", onChange, key, id }) {
  const unitOne = unit.endsWith("s") ? unit.slice(0, -1) : unit;
  const root = h("div", {
    class: "kp-quantity",
    id,
    "data-kp-quantity": "",
    "data-kp-quantity-fill": "",
    "data-kp-quantity-min": min,
    "data-kp-quantity-max": max,
    "data-kp-quantity-step": step,
    "data-kp-quantity-unit": unit,
    "data-kp-quantity-unit-one": unitOne,
    "data-kp-quantity-label": label,
    "data-key": key,
  }, h("input", { class: "kp-quantity__input", autocomplete: "off", value: String(value) }));
  if (onChange) root.addEventListener(QUANTITY_EVENT, (e) => onChange(/** @type {CustomEvent} */ (e).detail.value));
  return root;
}

/** Attach every stepper under `scope` (after it is in the document). @param {ParentNode} scope */
export const wire = (scope) => attachQuantities(scope);

/** The current number of a stepper. @param {HTMLElement} root */
export const stepperValue = (root) => getQuantity(root);

/**
 * The package's switch with its label; an empty `label` draws the switch alone (then `name` is its accessible name).
 * @param {{ label: string, checked: boolean, onChange: (on: boolean, input: HTMLInputElement) => void, key?: string, id?: string, describedby?: string, name?: string }} o
 */
export function toggle({ label, checked, onChange, key, id, describedby, name }) {
  const input = /** @type {HTMLInputElement} */ (h("input", { class: "kp-switch__input", type: "checkbox", role: "switch", id, "data-key": key, "aria-describedby": describedby, "aria-label": name }));
  input.checked = checked;
  input.addEventListener("change", () => onChange(input.checked, input));
  return h("label", { class: "kp-switch", "data-kp-switch-state": "none" }, input, label ? h("span", null, label) : null);
}

/**
 * One chip group (single choice), the package's segmented chips. With `none` the
 * chosen chip can be pressed off again: nothing chosen is a valid answer and `onChange` gets "".
 * @param {{ label: string, options: [string, string, number?][], value: string, onChange: (v: string) => void, none?: boolean }} o
 */
export function chips({ label, options, value, onChange, none = false }) {
  const group = h("div", { class: "kp-segmented kp-segmented--chips", "data-kp-segmented": none ? "single-or-none" : "single", role: "radiogroup", "aria-label": label },
    ...options.map(([v, text, count]) => h("button", { type: "button", class: "kp-segmented__item", role: "radio", "data-value": v, "aria-checked": String(v === value) }, h("span", { class: "kp-segmented__label" }, text), count === undefined ? null : h("span", { class: "kp-segmented__count" }, String(count)))));
  group.addEventListener("kp-segmented-change", (e) => {
    const detail = /** @type {CustomEvent} */ (e).detail;
    const v = none ? (detail?.values?.[0] ?? "") : detail?.value;
    if (typeof v === "string") onChange(v);
  });
  return group;
}

/**
 * Pills where several may be on (plain click toggles). `values` are the ones on.
 * With `linkedTo` (the id of a tag input) the package keeps pills and field in step both ways, so no `onChange` is needed.
 * @param {{ label: string, options: [string, string][], values: string[], onChange?: (values: string[]) => void, linkedTo?: string }} o
 */
export function chipsMulti({ label, options, values, onChange, linkedTo }) {
  const group = h("div", { class: "kp-segmented kp-segmented--chips", "data-kp-segmented": "multi", "data-kp-segmented-for": linkedTo, role: "group", "aria-label": label },
    ...options.map(([v, text]) => h("button", { type: "button", class: "kp-segmented__item", "data-value": v, "aria-pressed": String(values.includes(v)) }, h("span", { class: "kp-segmented__label" }, text))));
  if (onChange) {
    group.addEventListener("kp-segmented-change", (e) => {
      const list = /** @type {CustomEvent} */ (e).detail?.values;
      if (Array.isArray(list)) onChange(list);
    });
  }
  return group;
}
