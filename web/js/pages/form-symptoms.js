// The "Helps with" field of the medicine form: a creatable tag input for
// typing, and the existing symptoms below it as pills grouped by category.
// Tags and pills are one set: a pill toggles its tag, a removed tag lets go of
// its pill, and a name typed fresh simply becomes a new symptom on save.

import { attachSegmented } from "/static/kp/dist/kp-themes.min.js";
import { h } from "../dom.js";
import { chipsMulti } from "../controls.js";
import { row } from "./form-fields.js";

const OTHER = "Uncategorised";
const BOX_ID = "symptoms-box";

/** The tag input. @param {string[]} chosen @param {string[]} names */
function tagBox(chosen, names) {
  return h("div", { class: "kp-combobox", id: BOX_ID, "data-kp-combobox": "", "data-kp-tags": "", "data-kp-creatable": "", "data-kp-open-on-focus": "" },
    h("ul", { class: "kp-tag-list", "data-kp-tag-list": "" }, ...chosen.map((s) => h("li", { class: "kp-tag", "data-value": s }, h("span", null, s), h("button", { type: "button", class: "kp-tag__remove", "aria-label": `Remove ${s}` }, "×")))),
    h("input", { class: "kp-combobox__input", id: "symptoms", type: "text", role: "combobox", "aria-expanded": "false", "aria-controls": "symptoms-list", autocomplete: "off", placeholder: "Type a symptom, press Enter" }),
    h("ul", { class: "kp-combobox__list", id: "symptoms-list", role: "listbox", hidden: true }, ...names.map((s) => h("li", { class: "kp-combobox__option", role: "option", "data-kp-option": "", "data-value": s }, s))),
    h("p", { class: "kp-combobox__status", "data-kp-combobox-status": "", role: "status", "aria-live": "polite" }));
}

/**
 * @param {string[]} chosen the medicine's symptoms
 * @param {any[]} known GET /symptoms
 * @returns {{ box: HTMLElement, field: HTMLElement, wire: () => () => void }} `wire` runs once the form is in the page and its comboboxes are attached; it returns a detach
 */
export function symptomField(chosen, known) {
  const names = [...new Set([...known.map((s) => s.name), ...chosen])].sort();
  const box = tagBox(chosen, names);
  const groups = [...new Set(known.map((s) => s.category ?? OTHER))].sort((a, b) => (a === OTHER ? 1 : b === OTHER ? -1 : a.localeCompare(b)));
  // One pill group per category, each tied to the tag input: the package keeps pills and tags in step both ways.
  const pills = h("div", { class: "kp-autogrid kp-autogrid--tight" }, ...groups.map((g) => h("div", { class: "kp-stack kp-gap-xs" },
    h("span", { class: "kp-text-muted" }, g),
    chipsMulti({ label: `${g} symptoms`, values: chosen, linkedTo: BOX_ID, options: known.filter((s) => (s.category ?? OTHER) === g).map((s) => [s.name, s.name]) }))));
  const control = h("div", { class: "kp-stack" }, box, pills);
  return {
    box,
    field: row({ label: "Helps with", control, hint: "Type a new symptom and press Enter, or pick existing ones below, grouped by category.", id: "symptoms" }),
    wire: () => attachSegmented(pills),
  };
}
