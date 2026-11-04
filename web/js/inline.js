// Fields that edit in place: a text field or a select inside a row, saved when
// the person leaves it or presses Enter, with the server's refusal (message and
// remedy) shown right under it; and the three-dot menu of a row. Built from the
// package's field, select and menu-button parts.

import { attachAddRows, attachForms, attachMenuButtons, attachSelects, setMenu, ADD_EVENT, MENU_SELECT_EVENT } from "/static/kp/dist/kp-themes.min.js";
import { h } from "./dom.js";
import { errorText } from "./ui.js";

let serial = 0;

/** The line under a field that holds a refusal. @param {string} id */
const errorLine = (id) => h("p", { class: "kp-field__error", id, role: "alert", hidden: true });

/**
 * A field that saves itself. `onCommit(value)` resolves when the server accepted it; when it throws, the sentence stays under the field.
 * @param {{ kind: "text" | "select", label: string, value: string, onCommit: (value: string) => Promise<unknown>, required?: boolean, list?: string[], options?: [string, string][], key?: string, maxlength?: number }} f
 * @returns {{ el: HTMLElement, input: HTMLInputElement | HTMLSelectElement, set: (value: string) => void }}
 */
export function inlineField({ kind, label, value, onCommit, required = false, list, options = [], key, maxlength = 80 }) {
  const id = `inl${++serial}`;
  const err = errorLine(`${id}-e`);
  const input = /** @type {HTMLInputElement | HTMLSelectElement} */ (kind === "select"
    ? h("select", { class: "kp-field__input", id, "aria-label": label, "data-key": key }, ...options.map(([v, text]) => h("option", { value: v }, text)))
    : h("input", { class: "kp-field__input", id, type: "text", "aria-label": label, autocomplete: "off", maxlength, list: list ? `${id}-list` : null, "data-key": key }));
  input.value = value;
  let saved = value;
  let busy = false;
  const fail = (/** @type {string} */ text) => { err.textContent = text; err.hidden = false; input.setAttribute("aria-invalid", "true"); input.setAttribute("aria-describedby", err.id); };
  const clear = () => { err.hidden = true; err.textContent = ""; input.removeAttribute("aria-invalid"); input.removeAttribute("aria-describedby"); };
  async function commit() {
    const v = input.value.trim();
    if (busy || v === saved) return v === saved ? clear() : undefined;
    if (required && !v) return fail(`${label} is required. Type it, or press Escape to go back to “${saved}”.`);
    busy = true;
    try {
      await onCommit(v);
      saved = v;
      clear();
    } catch (e) {
      fail(errorText(e));
    } finally {
      busy = false;
    }
  }
  input.addEventListener(kind === "select" ? "change" : "blur", () => void commit());
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && kind === "text") { e.preventDefault(); void commit(); }
    if (e.key === "Escape" && kind === "text" && input.value !== saved) { input.value = saved; clear(); }
  });
  const el = h("div", { class: "kp-stack kp-gap-xs" }, input, list ? h("datalist", { id: `${id}-list` }, ...list.map((u) => h("option", { value: u }))) : null, err);
  return { el, input, set: (v) => { saved = v; input.value = v; clear(); } };
}

/** @typedef {{ label: string, value: string, danger?: boolean, run: () => void }} RowAction */
/** @type {WeakMap<HTMLElement, RowAction[]>} */
const menus = new WeakMap();

/** The three-dot menu of a row; it comes alive when `activate(host)` runs on the host it was put in. @param {string} label what the menu belongs to @param {RowAction[]} actions */
export function rowMenu(label, actions) {
  const id = `menu-${++serial}`;
  const wrapper = h("div", { class: "kp-menu-button", "data-kp-menu-button": "", "data-kp-key": id },
    h("button", { type: "button", class: "kp-icon-button", id, "aria-label": label }, "⋯"),
    h("div", { class: "kp-menu", role: "menu", "aria-labelledby": id, hidden: true }));
  menus.set(wrapper, actions);
  wrapper.addEventListener(MENU_SELECT_EVENT, (e) => actions.find((a) => a.value === /** @type {CustomEvent} */ (e).detail.value)?.run());
  return wrapper;
}

/** Rename a row menu's button and list after the row's name changed. @param {HTMLElement} wrapper @param {string} label */
export function relabelMenu(wrapper, label) {
  wrapper.querySelector("button")?.setAttribute("aria-label", label);
}

/** Wire the selects, row menus and add rows under `host`, once they are in the page. @param {HTMLElement} host @returns {() => void} detach */
export function activate(host) {
  const detach = [attachSelects(host), attachMenuButtons(host), attachForms(host), attachAddRows(host)];
  for (const w of /** @type {NodeListOf<HTMLElement>} */ (host.querySelectorAll("[data-kp-menu-button]"))) {
    const actions = menus.get(w);
    if (actions) setMenu(w, [{ group: "", items: actions.map((a) => ({ label: a.label, value: a.value, danger: a.danger })) }]);
  }
  return () => detach.forEach((d) => d());
}

/** One field of an add row: its label, the control and the line that holds its refusal. @param {string} fid @param {{ name: string, label: string, placeholder?: string, kind?: "text" | "select", options?: [string, string][], list?: string[], required?: boolean }} f */
function addField(fid, f) {
  const control = f.kind === "select"
    ? h("select", { class: "kp-field__input", id: fid, name: f.name }, ...(f.options ?? []).map(([v, text]) => h("option", { value: v }, text)))
    : h("input", { class: "kp-field__input", id: fid, name: f.name, type: "text", placeholder: f.placeholder, maxlength: 80, autocomplete: "off", required: f.required ? "" : null, list: f.list ? `${fid}-list` : null });
  return h("div", { class: "kp-field" },
    h("label", { class: "kp-field__label", for: fid }, f.label), control,
    f.list ? h("datalist", { id: `${fid}-list` }, ...f.list.map((u) => h("option", { value: u }))) : null,
    h("span", { class: "kp-field__error", "data-kp-field-error": "", hidden: true }));
}

/**
 * The row that adds an entry: the package's `.kp-addrow--line` (fields and the button on one line, the refusal under them).
 * `onAdd(values)` resolves when the entry exists; a refusal (message and remedy) is shown under the row and what was typed stays.
 * It comes alive when `activate(host)` runs on `el` or a host that holds it; `el` is the block of the form grid the line needs.
 * @param {{ label: string, hint: string, accept: string, fields: { name: string, label: string, placeholder?: string, kind?: "text" | "select", options?: [string, string][], list?: string[], required?: boolean }[], onAdd: (values: Record<string, string>) => Promise<unknown> }} o
 */
export function addRow({ label, hint, accept, fields, onAdd }) {
  const id = `add${++serial}`;
  const form = h("form", { class: "kp-addrow kp-addrow--line", "data-kp-addrow": "", "data-kp-form": "", novalidate: "", "aria-label": label },
    h("div", { class: "kp-form__summary", "data-kp-form-summary": "", tabindex: "-1", hidden: true }),
    h("div", { class: "kp-formgrid__row" },
      h("span", { class: "kp-formgrid__label" }, label),
      h("div", { class: "kp-formgrid__control kp-addrow__fields" }, ...fields.map((f, i) => addField(`${id}-${i}`, f))),
      h("div", { class: "kp-formgrid__actions" }, h("button", { type: "submit", class: "kp-button kp-button--primary", "data-kp-submit": "" }, accept))),
    h("p", { class: "kp-field__help kp-m-0" }, hint));
  const el = h("div", { class: "kp-formgrid" }, form);
  form.addEventListener(ADD_EVENT, (e) => {
    const { values, waitUntil } = /** @type {CustomEvent} */ (e).detail;
    waitUntil(onAdd(trimmed(values)).then(() => undefined, (err) => errorText(err)));
  });
  return { el, focus: () => /** @type {HTMLElement} */ (el.querySelector("input, select")).focus() };
}

/** The values of an add row, trimmed. @param {Record<string, unknown>} values */
const trimmed = (values) => Object.fromEntries(Object.entries(values).map(([k, v]) => [k, String(v ?? "").trim()]));
