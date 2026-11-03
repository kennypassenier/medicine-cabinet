// Fields that edit in place: a text field or a select inside a row, saved when
// the person leaves it or presses Enter, with the server's refusal (message and
// remedy) shown right under it; and the three-dot menu of a row. Built from the
// package's field, select and menu-button parts.

import { attachMenuButtons, attachSelects, setMenu, MENU_SELECT_EVENT } from "/static/kp/dist/kp-themes.min.js";
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

/** Wire the selects and the row menus under `host`, once they are in the page. @param {HTMLElement} host @returns {() => void} detach */
export function activate(host) {
  const detach = [attachSelects(host), attachMenuButtons(host)];
  for (const w of /** @type {NodeListOf<HTMLElement>} */ (host.querySelectorAll("[data-kp-menu-button]"))) {
    const actions = menus.get(w);
    if (actions) setMenu(w, [{ group: "", items: actions.map((a) => ({ label: a.label, value: a.value, danger: a.danger })) }]);
  }
  return () => detach.forEach((d) => d());
}

/**
 * The row that adds an entry: the fields and the button on one line, with the server's refusal under them.
 * `onAdd(values)` resolves when the entry exists; the fields are then emptied and the first one has the focus again.
 * @param {{ label: string, hint: string, accept: string, fields: { name: string, label: string, placeholder?: string, kind?: "text" | "select", options?: [string, string][], list?: string[], required?: boolean }[], onAdd: (values: Record<string, string>) => Promise<unknown> }} o
 */
export function addRow({ label, hint, accept, fields, onAdd }) {
  const id = `add${++serial}`;
  const err = errorLine(`${id}-e`);
  const inputs = fields.map((f, i) => {
    const fid = `${id}-${i}`;
    const el = /** @type {HTMLInputElement | HTMLSelectElement} */ (f.kind === "select"
      ? h("select", { class: "kp-field__input", id: fid, "aria-label": f.label }, ...(f.options ?? []).map(([v, text]) => h("option", { value: v }, text)))
      : h("input", { class: "kp-field__input", id: fid, type: "text", "aria-label": f.label, placeholder: f.placeholder, maxlength: 80, autocomplete: "off", list: f.list ? `${fid}-list` : null }));
    return { f, el, list: f.list ? h("datalist", { id: `${fid}-list` }, ...f.list.map((u) => h("option", { value: u }))) : null };
  });
  const go = h("button", { type: "button", class: "kp-button kp-button--primary" }, accept);
  const clearInvalid = () => inputs.forEach(({ el }) => { el.removeAttribute("aria-invalid"); el.removeAttribute("aria-describedby"); });
  async function submit() {
    if (go.hasAttribute("aria-busy")) return; // a double tap must not send the entry twice
    const values = Object.fromEntries(inputs.map(({ f, el }) => [f.name, el.value.trim()]));
    const missing = inputs.find(({ f, el }) => f.required && !el.value.trim());
    clearInvalid();
    if (missing) { err.textContent = `${missing.f.label} is required. Fill it in, then press ${accept}.`; err.hidden = false; missing.el.setAttribute("aria-invalid", "true"); missing.el.setAttribute("aria-describedby", err.id); return missing.el.focus(); }
    go.setAttribute("aria-busy", "true");
    try {
      await onAdd(values);
      for (const { f, el } of inputs) if (f.kind !== "select") el.value = "";
      err.hidden = true;
      inputs[0].el.focus();
    } catch (e) {
      err.textContent = errorText(e);
      err.hidden = false;
      inputs[0].el.setAttribute("aria-describedby", err.id);
    } finally {
      go.removeAttribute("aria-busy");
    }
  }
  go.addEventListener("click", () => void submit());
  for (const { el } of inputs) el.addEventListener("keydown", (e) => { if (e.key === "Enter" && el instanceof HTMLInputElement) { e.preventDefault(); void submit(); } });
  const grid = h("div", { class: "kp-autogrid kp-autogrid--tight mc-inline-grid" }, ...inputs.map(({ el: i }) => i), go);
  const el = h("div", { class: "kp-stack kp-gap-xs", role: "group", "aria-labelledby": `${id}-t` },
    h("strong", { id: `${id}-t` }, label), grid, ...inputs.map(({ list }) => list), err, h("p", { class: "kp-field__help kp-m-0" }, hint));
  return { el, focus: () => inputs[0].el.focus() };
}
