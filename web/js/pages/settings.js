// Settings: the medicine forms (Tablet, Syrup…) and the unit each one counts in,
// edited in place. Name and unit save when you leave the field; Add is a row at
// the top; Delete is in the row's menu and offers Undo (the medicines keep their form).

import { h } from "../dom.js";
import { api } from "../api.js";
import { drawEntityTable } from "../entity-table.js";
import { activate, addRow, inlineField, relabelMenu, rowMenu } from "../inline.js";
import { refreshNow, REFRESH_EVENT } from "../ticker.js";
import { COMMON_UNITS } from "../units.js";
import { undoable } from "../undo.js";
import { dataCard, errorText, pageHeader, reportError } from "../ui.js";

/**
 * @typedef {{
 *   forms: any[], alive: boolean, detach: (() => void)[], card: ReturnType<typeof dataCard>, table: HTMLElement, add: ReturnType<typeof addRowFor>
 * }} SettingsPage
 */

/** The form's row in the table: two editable fields, how many medicines use it, and its menu. @param {SettingsPage} s @param {any} f */
function formRow(s, f) {
  const cur = { name: f.name, unit: f.unit };
  const save = (/** @type {Partial<typeof cur>} */ patch) => api.editForm(f.id, { ...cur, ...patch }).then(() => { Object.assign(cur, patch); Object.assign(f, patch); });
  const name = inlineField({ kind: "text", label: `Name of the form ${f.name}`, value: f.name, required: true, key: `form-name-${f.id}`, onCommit: async (v) => { await save({ name: v }); name.input.closest("td")?.setAttribute("data-kp-sort-value", v); name.input.setAttribute("aria-label", `Name of the form ${v}`); unit.input.setAttribute("aria-label", `Unit of ${v}`); relabelMenu(menu, `Actions for the form ${v}`); } });
  const unit = inlineField({ kind: "text", label: `Unit of ${f.name}`, value: f.unit, required: true, list: COMMON_UNITS, key: `form-unit-${f.id}`, onCommit: async (v) => { await save({ unit: v }); unit.input.closest("td")?.setAttribute("data-kp-sort-value", v); } });
  const menu = rowMenu(`Actions for the form ${f.name}`, [{ label: "Delete", value: "delete", danger: true, run: () => void remove(s, f) }]);
  return { key: `f${f.id}`, cells: [name.el, unit.el, f.medicines, menu], sortValues: [f.name, f.unit, f.medicines, undefined] };
}

/** Delete at once; Undo creates it again with the medicines that had it. @param {SettingsPage} s @param {any} f */
async function remove(s, f) {
  let gone;
  try {
    gone = await api.removeForm(f.id);
  } catch (e) {
    return reportError(`Could not delete ${f.name}.`, e);
  }
  await load(s);
  const n = gone.was.medicine_ids.length;
  const note = n ? ` ${n} ${n === 1 ? "medicine keeps" : "medicines keep"} no form.` : "";
  undoable(`${f.name} deleted.${note}`, { undo: () => api.addForm(gone.was), failed: `Could not bring ${f.name} back.`, after: refreshNow });
}

/** Draw the table from the forms at hand. @param {SettingsPage} s */
function draw(s) {
  s.detach.forEach((d) => d());
  if (!s.forms.length) return s.card.state("empty", { title: "No forms yet", text: "Add the first form above, for example Tablet counted in tablets." });
  const stop = drawEntityTable(s.table, {
    label: "Medicine forms", none: "No form matches.", pageSize: 12,
    columns: [{ title: "Form" }, { title: "Unit" }, { title: "Medicines", sort: "number", numeric: true }, { title: "Actions", actions: true }],
    rows: s.forms.map((f) => formRow(s, f)),
  });
  s.detach = [stop, activate(s.table)];
  s.card.state("ready");
}

async function load(/** @type {SettingsPage} */ s) {
  try {
    s.forms = await api.forms();
    if (s.alive) draw(s);
  } catch (e) {
    if (!s.alive) return;
    if (!s.forms.length) s.card.state("error", { title: "The forms could not be loaded", text: errorText(e) });
    else reportError("The forms could not be refreshed.", e);
  }
}

/** The row that adds a form. @param {() => Promise<void>} reload */
const addRowFor = (reload) => addRow({
  label: "Add a form", hint: "The name of how a medicine comes, and what it is counted in.", accept: "Add form",
  fields: [{ name: "name", label: "Name of the new form", placeholder: "e.g. Lozenge", required: true }, { name: "unit", label: "Unit of the new form", placeholder: "e.g. lozenges", list: COMMON_UNITS, required: true }],
  onAdd: async (v) => { await api.addForm({ name: v.name, unit: v.unit }); await reload(); },
});

/** @param {HTMLElement} root */
export function mount(root) {
  document.title = "Settings · Medicine cabinet";
  const card = dataCard({ id: "forms", title: "Medicine forms", description: "How a medicine comes (tablet, syrup, patch…) and the unit its stock is counted in. Edit a name or unit in place; it is saved when you leave the field.", onRetry: () => void load(s) });
  card.card.dataset.kpSpan = "12";
  const table = h("div");
  /** @type {SettingsPage} */
  const s = { forms: [], alive: true, detach: [], card, table, add: addRowFor(() => load(s)) };
  card.data.append(table);
  card.card.querySelector(".kp-card__stage")?.before(s.add.el);
  root.append(pageHeader({ title: "Settings", description: "Lookups the cabinet uses. Changes are saved as you make them." }), h("div", { class: "kp-ops__body" }, card.card));
  card.state("loading");
  void load(s);
  // Only an Undo asks for a reload: a timer would redraw the rows under a field being edited.
  const again = () => { if (!table.contains(document.activeElement)) void load(s); };
  window.addEventListener(REFRESH_EVENT, again);
  return () => { s.alive = false; s.detach.forEach((d) => d()); window.removeEventListener(REFRESH_EVENT, again); };
}
