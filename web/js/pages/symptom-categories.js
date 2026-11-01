// The category pills of the Symptoms page. A pill only filters the symptom
// list; each category has its own three-dot menu beside its pill (Rename,
// Delete). Rename edits the pill's label in place. "All" and "Uncategorised"
// are not categories and have no menu.

import { h } from "../dom.js";
import { chips } from "../controls.js";
import { activate, rowMenu } from "../inline.js";
import { errorText } from "../ui.js";

export const UNCATEGORISED = "Uncategorised";

/**
 * Put `input` where `chip` is, until the name is saved or abandoned. A refusal stays in `fail` and keeps the field open.
 * @param {HTMLElement} chip @param {any} cat @param {(name: string) => Promise<unknown>} rename @param {(text: string) => void} fail
 */
function startRename(chip, cat, rename, fail) {
  const input = /** @type {HTMLInputElement} */ (h("input", { class: "kp-field__input", type: "text", value: cat.name, maxlength: 80, "aria-label": `New name for the category ${cat.name}`, autocomplete: "off" }));
  chip.hidden = true;
  chip.after(input);
  input.focus();
  input.select();
  let busy = false;
  const close = () => { input.remove(); chip.hidden = false; chip.focus(); };
  async function commit() {
    const name = input.value.trim();
    if (busy) return;
    if (name === cat.name) return close();
    if (!name) return fail("A category needs a name. Type one, or press Escape to keep the old name.");
    busy = true;
    try {
      await rename(name);
      close();
    } catch (e) {
      fail(errorText(e));
    } finally {
      busy = false;
    }
  }
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter") { e.preventDefault(); void commit(); }
    if (e.key === "Escape") { fail(""); close(); }
  });
  input.addEventListener("blur", () => { if (input.isConnected) void commit(); });
}

/**
 * @param {HTMLElement} host replaced by the pill group
 * @param {{ categories: any[], symptoms: any[], filter: string, onFilter: (v: string) => void, rename: (cat: any, name: string) => Promise<unknown>, remove: (cat: any) => void, fail: (text: string) => void }} o
 * @returns {() => void} detach
 */
export function drawCategoryPills(host, { categories, symptoms, filter, onFilter, rename, remove, fail }) {
  const unc = symptoms.filter((s) => s.category_id === null).length;
  const options = /** @type {[string, string, number][]} */ ([["all", "All", symptoms.length], ...categories.map((c) => [String(c.id), c.name, symptoms.filter((s) => s.category_id === c.id).length]), ...(unc || filter === "none" ? [["none", UNCATEGORISED, unc]] : [])]);
  const group = chips({ label: "Show category", options, value: filter, onChange: onFilter });
  group.setAttribute("data-mc-pills", "");
  for (const chip of group.querySelectorAll(".kp-segmented__item")) chip.toggleAttribute("data-mc-solo", !categories.some((c) => String(c.id) === /** @type {HTMLElement} */ (chip).dataset.value));
  for (const c of categories) {
    const chip = /** @type {HTMLElement} */ (group.querySelector(`[data-value="${c.id}"]`));
    chip.after(rowMenu(`Actions for the category ${c.name}`, [
      { label: "Rename", value: "rename", run: () => setTimeout(() => startRename(chip, c, (name) => rename(c, name), fail), 0) },
      { label: "Delete", value: "delete", danger: true, run: () => remove(c) },
    ]));
  }
  host.replaceChildren(group);
  return activate(host);
}
