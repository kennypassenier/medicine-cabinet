// "Manage forms": the list of medicine forms (Tablet, Syrup…) with the unit
// each one counts in. Full add / edit / delete in one dialog; it resolves when
// closed, true when anything changed, so the caller can reload its forms.

import { h } from "../dom.js";
import { api } from "../api.js";
import { ask } from "../dialogs.js";
import { drawEntityTable, rowButton } from "../entity-table.js";
import { formDialog } from "../form-dialog.js";
import { button, errorText, reportError, say } from "../ui.js";

export const COMMON_UNITS = ["tablets", "capsules", "ml", "drops", "g", "doses", "puffs", "patches", "sachets"];

/** @param {any | null} f the form being edited, or null for a new one */
function editForm(f) {
  return formDialog({
    title: f ? `Edit ${f.name}` : "Add a form",
    description: f ? "The unit is what a new medicine of this form is counted in." : "How a medicine comes: tablet, syrup, patch…",
    accept: f ? "Save changes" : "Add form",
    fields: [
      { name: "name", label: "Name", required: true, value: f?.name, hint: "e.g. Lozenge" },
      { name: "unit", label: "Unit", required: true, value: f?.unit, list: COMMON_UNITS, hint: "Counted in: tablets, ml, drops…" },
    ],
    save: async (v) => { await (f ? api.editForm(f.id, v) : api.addForm(v)); say(`${v.name} ${f ? "saved" : "added"}.`); },
  });
}

/** @param {any} f */
async function removeForm(f) {
  const text = f.medicines
    ? `${f.name} is the form of ${f.medicines} ${f.medicines === 1 ? "medicine" : "medicines"}. They stay in your cabinet, but lose their form.`
    : `${f.name} is not used by any medicine. It is simply removed.`;
  if (!(await ask({ title: `Delete ${f.name}?`, text, accept: `Delete ${f.name}`, destructive: true }))) return false;
  try {
    await api.removeForm(f.id);
    say(`${f.name} deleted.`);
    return true;
  } catch (e) {
    reportError(`Could not delete ${f.name}.`, e);
    return false;
  }
}

/**
 * The dialog's shared state.
 * @typedef {{ changed: boolean, detach: () => void, ui: ReturnType<typeof buildDialog> }} Manager
 */

/** The dialog with its table host and error line; the callbacks wire the two buttons. @param {{ add: () => void, done: () => void }} on */
function buildDialog(on) {
  const host = h("div", { class: "mc-formsdlg__table" });
  const status = h("p", { class: "kp-alert kp-alert--destructive", role: "alert", hidden: true });
  const dialog = /** @type {HTMLDialogElement} */ (h("dialog", { class: "kp-dialog mc-dialog mc-dialog--wide", "aria-labelledby": "forms-t", "aria-describedby": "forms-d" },
    h("h2", { class: "kp-dialog__title", id: "forms-t" }, "Manage forms"),
    h("p", { class: "kp-dialog__description", id: "forms-d" }, "The forms a medicine can have, and the unit each one is counted in."),
    host, status,
    h("div", { class: "kp-dialog__actions" },
      button("Add form", { onClick: on.add }),
      button("Done", { look: "kp-button--primary", onClick: on.done }))));
  return { dialog, host, status };
}

/** Run an edit flow; redraw the table when it changed something. @param {Manager} d @param {Promise<boolean>} done */
async function run(d, done) {
  if (await done) {
    d.changed = true;
    await draw(d);
  }
}

/** Edit and Delete of one form, in one fixed cell. @param {Manager} d @param {any} f */
const formActions = (d, f) => h("span", { class: "mc-actions__pair" },
  rowButton({ label: "Edit", aria: `Edit form ${f.name}`, onClick: () => run(d, editForm(f)) }),
  rowButton({ label: "Delete", aria: `Delete form ${f.name}`, look: "kp-button--destructive", onClick: () => run(d, removeForm(f)) }));

/** @param {Manager} d */
async function draw(d) {
  const { host, status } = d.ui;
  try {
    const forms = await api.forms();
    status.hidden = true;
    d.detach();
    d.detach = drawEntityTable(host, {
      label: "Forms", none: "No form matches.", pageSize: 10,
      columns: [{ title: "Form" }, { title: "Unit" }, { title: "Medicines", sort: "number", numeric: true }, { title: "Actions", actions: true }],
      rows: forms.map((/** @type {any} */ f) => ({ key: `f${f.id}`, cells: [f.name, f.unit, f.medicines, formActions(d, f)] })),
    });
  } catch (e) {
    status.textContent = errorText(e);
    status.hidden = false;
  }
}

/** @returns {Promise<boolean>} true when a form was added, changed or deleted */
export function openFormsManager() {
  return new Promise((resolve) => {
    /** @type {Manager} */
    const d = {
      changed: false,
      detach: () => {},
      ui: buildDialog({ add: () => void run(d, editForm(null)), done: () => d.ui.dialog.close() }),
    };
    const { dialog } = d.ui;
    dialog.addEventListener("close", () => { d.detach(); dialog.remove(); resolve(d.changed); });
    document.body.append(dialog);
    dialog.showModal();
    void draw(d);
  });
}
