// Field builders for the medicine form: package markup in `.kp-formgrid`
// rows (label | control | hint), so every field of every card lines up.

import { h } from "../dom.js";
import { formatDate } from "../clock.js";

let serial = 0;
const uid = () => `f${++serial}`;

/**
 * One form-grid row.
 * @param {{ label: string, control: HTMLElement, hint?: string, id?: string, required?: boolean }} r
 */
export function row({ label, control, hint, id, required = false }) {
  const hintId = hint && id ? `${id}-hint` : undefined;
  const field = id ? control.querySelector(`#${id}`) ?? (control.id === id ? control : null) : null;
  if (hintId) field?.setAttribute("aria-describedby", hintId);
  return h("div", { class: "kp-formgrid__row", "data-kp-row": "", "data-kp-name": label },
    h("label", { class: "kp-formgrid__label", for: id }, label, required ? h("span", { class: "kp-field__required" }, " required") : null),
    h("div", { class: "kp-formgrid__control" }, control, h("span", { class: "kp-field__error", "data-kp-field-error": "", hidden: true })),
    hint ? h("p", { class: "kp-formgrid__value", id: hintId }, hint) : null);
}

/** @param {{ name: string, label: string, value?: string, hint?: string, required?: boolean, multiline?: boolean, list?: string, placeholder?: string, maxlength?: number }} o */
export function text({ name, label, value = "", hint, required, multiline, list, placeholder, maxlength }) {
  const id = uid();
  const attrs = { id, name, required, list, placeholder, maxlength, autocomplete: "off" };
  const input = multiline
    ? h("textarea", { ...attrs, class: "kp-field__input kp-field__input--multiline", rows: 3 })
    : h("input", { ...attrs, class: "kp-field__input", type: "text" });
  /** @type {HTMLInputElement} */ (input).value = value;
  return row({ label, control: input, hint, id, required });
}

/** The package's date picker alone (ISO in, `dataset.kpDateValue` out); `max` is the last day it accepts. @param {{ name: string, label: string, id: string, value?: string | null, max?: string }} o */
export function datePicker({ name, label, id, value, max }) {
  return h("div", { class: "kp-datepicker", "data-kp-datepicker": "", lang: "en-GB", "data-name": name, "data-kp-max": max },
    h("input", { class: "kp-field__input", id, type: "text", inputmode: "numeric", autocomplete: "off", "data-kp-date-input": "", placeholder: "dd/mm/yyyy", "aria-label": label, value: value ? formatDate(value) : null }),
    h("button", { type: "button", class: "kp-button", "data-kp-date-open": "", "aria-label": `Open the calendar for ${label}`, title: "Open the calendar" }, h("span", { "aria-hidden": "true" }, "▦")),
    h("div", { class: "kp-datepicker__panel", "data-kp-date-panel": "", hidden: true }));
}

/** A date row of a form; value ISO in, ISO out. @param {{ name: string, label: string, value?: string | null, hint?: string }} o */
export function date({ name, label, value, hint }) {
  const id = uid();
  return row({ label, control: datePicker({ name, label, id, value }), hint, id });
}

/** The ISO value of a named date picker in `scope`, or null. @param {ParentNode} scope @param {string} name */
export function dateValue(scope, name) {
  const input = scope.querySelector(`[data-name="${name}"] input`);
  return input instanceof HTMLInputElement ? input.dataset.kpDateValue || null : null;
}

/** A card section of a form: heading, one sentence, then its rows; `span` is its width in the page's 12 columns. @param {{ id: string, title: string, description: string, rows: (HTMLElement | null)[], span?: number }} s */
export function section({ id, title, description, rows, span = 6 }) {
  return h("section", { class: "kp-card", "data-kp-span": span, "aria-labelledby": `${id}-t` },
    h("div", { class: "kp-card__header" }, h("h2", { class: "kp-card__title", id: `${id}-t` }, title)),
    h("p", { class: "kp-card__body" }, description),
    h("div", { class: "kp-formgrid" }, ...rows.filter(Boolean)));
}
