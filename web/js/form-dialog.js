// A small form in a dialog (name, a unit, a category pill…) built from the
// package's dialog, form grid, field and chip parts. The server's own refusal
// (message + remedy) stays in the dialog as an alert until the entry is right.

import { h } from "./dom.js";
import { chips } from "./controls.js";
import { errorText } from "./ui.js";

let serial = 0;

/**
 * One field row of the dialog.
 * @param {{ name: string, label: string, value?: string, hint?: string, required?: boolean, list?: string[], choices?: [string, string][] }} f
 * @param {string} uid
 * @returns {{ row: HTMLElement, read: () => string }}
 */
function fieldRow(f, uid) {
  const id = `${uid}-${f.name}`;
  let control;
  /** @type {() => string} */
  let read;
  if (f.choices) {
    let chosen = f.value ?? f.choices[0][0];
    control = chips({ label: f.label, options: f.choices, value: chosen, onChange: (v) => { chosen = v; } });
    read = () => chosen;
  } else {
    const input = /** @type {HTMLInputElement} */ (h("input", { class: "kp-field__input", type: "text", id, name: f.name, required: f.required, autocomplete: "off", maxlength: 80, list: f.list ? `${id}-list` : null, value: f.value ?? "" }));
    control = f.list ? h("div", null, input, h("datalist", { id: `${id}-list` }, ...f.list.map((u) => h("option", { value: u })))) : input;
    read = () => input.value.trim();
  }
  const row = h("div", { class: "kp-formgrid__row" },
    h("label", { class: "kp-formgrid__label", for: id }, f.label),
    h("div", { class: "kp-formgrid__control" }, control),
    f.hint ? h("p", { class: "kp-formgrid__value" }, f.hint) : null);
  return { row, read };
}

/**
 * Ask for a few values. `save` runs on submit; when it throws, its sentence is
 * shown in the dialog and nothing closes. Resolves true after a save, else false.
 * @param {{ title: string, description: string, accept: string, fields: Parameters<typeof fieldRow>[0][], save: (values: Record<string, string>) => Promise<void> }} o
 */
export function formDialog({ title, description, accept, fields, save }) {
  return new Promise((resolve) => {
    const uid = `fd${++serial}`;
    const rows = fields.map((f) => ({ f, ...fieldRow(f, uid) }));
    const alert = h("div", { class: "kp-alert kp-alert--destructive", role: "alert", hidden: true });
    const cancel = h("button", { type: "button", class: "kp-button" }, "Cancel");
    const submit = h("button", { type: "submit", class: "kp-button kp-button--primary" }, accept);
    const form = h("form", { class: "mc-dialog__form", novalidate: true },
      h("div", { class: "kp-formgrid" }, ...rows.map((r) => r.row)),
      alert,
      h("div", { class: "kp-dialog__actions" }, cancel, submit));
    const dialog = /** @type {HTMLDialogElement} */ (h("dialog", { class: "kp-dialog mc-dialog mc-dialog--form", "aria-labelledby": `${uid}-t`, "aria-describedby": `${uid}-d` },
      h("h2", { class: "kp-dialog__title", id: `${uid}-t` }, title),
      h("p", { class: "kp-dialog__description", id: `${uid}-d` }, description),
      form));
    let saved = false;
    /** @param {string} text */
    const fail = (text) => {
      alert.replaceChildren(h("div", { class: "kp-alert__body" }, text));
      alert.hidden = false;
    };
    form.addEventListener("submit", async (e) => {
      e.preventDefault();
      const values = Object.fromEntries(rows.map((r) => [r.f.name, r.read()]));
      const missing = rows.find((r) => r.f.required && !values[r.f.name]);
      if (missing) return fail(`${missing.f.label} is required. Type it, then try again.`);
      submit.disabled = true;
      try {
        await save(values);
        saved = true;
        dialog.close();
      } catch (err) {
        submit.disabled = false;
        fail(errorText(err));
      }
    });
    cancel.addEventListener("click", () => dialog.close());
    dialog.addEventListener("close", () => { dialog.remove(); resolve(saved); });
    document.body.append(dialog);
    dialog.showModal();
    dialog.querySelector("input")?.focus();
  });
}
