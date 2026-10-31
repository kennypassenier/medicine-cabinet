// The cabinet's search and filters: which medicines are shown, the two chip
// groups that choose them, and the URL that remembers the choice.

import { h } from "../dom.js";
import { chips } from "../controls.js";

/** The filters, by key. */
export const FILTERS = /** @type {const} */ ({
  all: ["All", () => true],
  on: ["Switched on", (/** @type {any} */ m) => m.active],
  off: ["Switched off", (/** @type {any} */ m) => !m.active],
  low: ["Low or out", (/** @type {any} */ m) => m.stock_level !== "ok"],
  expiring: ["Expiring", (/** @type {any} */ m) => m.freshness !== "fresh"],
});

/** @typedef {{ query: string, filter: keyof typeof FILTERS, formFilter: string }} FilterView */

const haystack = (/** @type {any} */ m) => [m.name, m.purpose, m.form, m.strength, ...m.symptoms].join(" ").toLowerCase();

/** The view a page load starts from: whatever the URL remembers. @returns {FilterView} */
export function readView() {
  const params = new URLSearchParams(location.search);
  const f = params.get("f");
  return {
    query: params.get("q") ?? "",
    filter: /** @type {keyof typeof FILTERS} */ (f !== null && f in FILTERS ? f : "all"),
    formFilter: params.get("form") ?? "",
  };
}

/** Write the view into the address bar, without a history entry. @param {FilterView} view */
export function syncUrl(view) {
  const p = new URLSearchParams();
  if (view.query) p.set("q", view.query);
  if (view.filter !== "all") p.set("f", view.filter);
  if (view.formFilter) p.set("form", view.formFilter);
  history.replaceState(null, "", p.size ? `?${p}` : location.pathname);
}

/** The medicines the view lets through, by name. @param {any[]} meds @param {FilterView} view */
export function visibleMedicines(meds, view) {
  const q = view.query.trim().toLowerCase();
  return meds
    .filter((m) => FILTERS[view.filter][1](m) && (!view.formFilter || String(m.form_id) === view.formFilter) && (!q || haystack(m).includes(q)))
    .sort((a, b) => a.name.localeCompare(b.name));
}

/**
 * Redraw both chip groups from the medicines at hand.
 * @param {{ filterSlot: HTMLElement, formSlot: HTMLElement }} slots
 * @param {any[]} meds
 * @param {FilterView} view mutated when a chosen form no longer exists
 * @param {() => void} onChange runs after the view changed, the URL is already updated
 */
export function drawFilterChips(slots, meds, view, onChange) {
  slots.filterSlot.replaceChildren(h("b", { class: "kp-toolbar__label" }, "Show"), chips({
    label: "Show",
    value: view.filter,
    options: Object.entries(FILTERS).map(([k, [label, fn]]) => [k, label, meds.filter(fn).length]),
    onChange: (v) => {
      view.filter = /** @type {keyof typeof FILTERS} */ (v);
      syncUrl(view);
      onChange();
    },
  }));
  drawFormChips(slots.formSlot, meds, view, onChange);
}

/** The form pills: only forms some medicine has, with how many. */
function drawFormChips(/** @type {HTMLElement} */ formSlot, /** @type {any[]} */ meds, /** @type {FilterView} */ view, /** @type {() => void} */ onChange) {
  const names = [...new Map(meds.filter((m) => m.form_id).map((m) => [String(m.form_id), m.form])).entries()].sort((a, b) => a[1].localeCompare(b[1]));
  if (view.formFilter && !names.some(([id]) => id === view.formFilter)) view.formFilter = "";
  formSlot.hidden = names.length === 0;
  formSlot.replaceChildren(h("b", { class: "kp-toolbar__label" }, "Form"), chips({
    label: "Form",
    value: view.formFilter,
    options: [["", "Any form", meds.length], ...names.map(([id, name]) => /** @type {[string, string, number]} */ ([id, name, meds.filter((m) => String(m.form_id) === id).length]))],
    onChange: (v) => { view.formFilter = v; syncUrl(view); onChange(); },
  }));
}
