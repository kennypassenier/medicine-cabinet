// The "Helps with" field of the medicine form: a creatable tag input for
// typing, and the existing symptoms below it as pills grouped by category.
// Tags and pills are one set: a pill toggles its tag, a removed tag lets go of
// its pill, and a name typed fresh simply becomes a new symptom on save.

import { combobox } from "/static/kp/js/combobox.js";
import { h } from "../dom.js";
import { chipsMulti } from "../controls.js";
import { row } from "./form-fields.js";

const OTHER = "Other";

/** The tag input. @param {string[]} chosen @param {string[]} names */
function tagBox(chosen, names) {
  return h("div", { class: "kp-combobox", "data-kp-combobox": "", "data-kp-tags": "", "data-kp-creatable": "", "data-kp-open-on-focus": "" },
    h("ul", { class: "kp-tag-list", "data-kp-tag-list": "" }, ...chosen.map((s) => h("li", { class: "kp-tag", "data-value": s }, h("span", null, s), h("button", { type: "button", class: "kp-tag__remove", "aria-label": `Remove ${s}` }, "×")))),
    h("input", { class: "kp-combobox__input", id: "symptoms", type: "text", role: "combobox", "aria-expanded": "false", "aria-controls": "symptoms-list", autocomplete: "off", placeholder: "Type a symptom, press Enter" }),
    h("ul", { class: "kp-combobox__list", id: "symptoms-list", role: "listbox", hidden: true }, ...names.map((s) => h("li", { class: "kp-combobox__option", role: "option", "data-kp-option": "", "data-value": s }, s))),
    h("p", { class: "kp-combobox__status", "data-kp-combobox-status": "", role: "status", "aria-live": "polite" }));
}

/**
 * @param {string[]} chosen the medicine's symptoms
 * @param {any[]} known GET /symptoms
 * @returns {{ box: HTMLElement, field: HTMLElement, wire: () => void }} `wire` runs once the form is in the page and its comboboxes are attached
 */
export function symptomField(chosen, known) {
  const names = [...new Set([...known.map((s) => s.name), ...chosen])].sort();
  const box = tagBox(chosen, names);
  const pills = h("div", { class: "mc-pickgroups" });
  const control = h("div", { class: "mc-pick" }, box, pills);
  const groups = [...new Set(known.map((s) => s.category ?? OTHER))].sort((a, b) => (a === OTHER ? 1 : b === OTHER ? -1 : a.localeCompare(b)));

  /** @param {string[]} values */
  function drawPills(values) {
    pills.replaceChildren(...groups.map((g) => h("div", { class: "mc-pickgroup" },
      h("span", { class: "mc-pickgroup__name" }, g),
      chipsMulti({ label: `${g} symptoms`, values, options: known.filter((s) => (s.category ?? OTHER) === g).map((s) => [s.name, s.name]), onChange: (on) => toggle(g, on) }))));
  }

  /** A pill group changed: keep tags of other groups and typed ones, replace this group's. @param {string} group @param {string[]} on */
  function toggle(group, on) {
    const handle = combobox(box);
    if (!handle) return;
    const inGroup = new Set(known.filter((s) => (s.category ?? OTHER) === group).map((s) => s.name));
    handle.set([...handle.values().filter((v) => !inGroup.has(v)), ...on]);
  }

  return {
    box,
    field: row({ label: "Helps with", control, hint: "Type a new symptom and press Enter, or pick existing ones below, grouped by category.", id: "symptoms" }),
    wire() {
      drawPills(combobox(box)?.values() ?? chosen);
      box.addEventListener("kp-combobox-change", (e) => {
        const now = /** @type {CustomEvent} */ (e).detail?.values;
        if (Array.isArray(now) && !pills.contains(document.activeElement)) drawPills(now);
      });
    },
  };
}
