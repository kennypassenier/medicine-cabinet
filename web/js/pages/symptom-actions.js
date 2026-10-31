// The add / rename / move / delete flows of symptoms and categories: each is a
// dialog, each ends by saying what changed. They resolve true when something
// was saved so the page reloads its lists.

import { api } from "../api.js";
import { ask } from "../dialogs.js";
import { formDialog } from "../form-dialog.js";
import { reportError, say } from "../ui.js";

const plural = (/** @type {number} */ n, /** @type {string} */ one, /** @type {string} */ many) => `${n} ${n === 1 ? one : many}`;

/** The category pills of a symptom dialog; none pressed means no category. @param {any[]} categories */
const categoryChoices = (categories) => /** @type {[string, string][]} */ (categories.map((c) => [String(c.id), c.name]));

/** @param {string} v */
const categoryId = (v) => (v ? Number(v) : null);

/** @param {any[]} categories @param {number | null} [preset] */
export function addSymptom(categories, preset = null) {
  return formDialog({
    title: "Add a symptom",
    description: "Name what you take medicine for. It shows up in the search and in the medicine form.",
    accept: "Add symptom",
    fields: [
      { name: "name", label: "Name", required: true, hint: "e.g. headache" },
      { name: "category", label: "Category", choices: categoryChoices(categories), none: true, value: preset ? String(preset) : "", hint: "Where it is listed." },
    ],
    save: async (v) => { await api.addSymptom({ name: v.name, category_id: categoryId(v.category) }); say(`${v.name.toLowerCase()} added.`); },
  });
}

/** @param {any} s @param {any[]} categories */
export function editSymptom(s, categories) {
  return formDialog({
    title: `Edit ${s.name}`,
    description: s.medicines ? `Renaming changes it on ${plural(s.medicines, "medicine", "medicines")} too.` : "No medicine uses it yet.",
    accept: "Save changes",
    fields: [
      { name: "name", label: "Name", required: true, value: s.name },
      { name: "category", label: "Category", choices: categoryChoices(categories), none: true, value: s.category_id ? String(s.category_id) : "", hint: "Move it to another group." },
    ],
    save: async (v) => { await api.editSymptom(s.id, { name: v.name, category_id: categoryId(v.category) }); say(`${v.name.toLowerCase()} saved.`); },
  });
}

/** @param {any} s */
export async function deleteSymptom(s) {
  const text = s.medicines
    ? `${s.name} is listed on ${plural(s.medicines, "medicine", "medicines")}. They stay in your cabinet, but no longer help with it, and searching for it finds nothing.`
    : `${s.name} is not used by any medicine. It is simply removed from the list.`;
  if (!(await ask({ title: `Delete ${s.name}?`, text, accept: `Delete ${s.name}`, destructive: true }))) return false;
  try {
    await api.removeSymptom(s.id);
    say(`${s.name} deleted.`);
    return true;
  } catch (e) {
    reportError(`Could not delete ${s.name}.`, e);
    return false;
  }
}

export function addCategory() {
  return formDialog({
    title: "Add a category",
    description: "A group such as Head or Stomach. Symptoms are listed under it.",
    accept: "Add category",
    fields: [{ name: "name", label: "Name", required: true, hint: "e.g. Head" }],
    save: async (v) => { await api.addCategory(v.name); say(`${v.name} added.`); },
  });
}

/** @param {any} c */
export function editCategory(c) {
  return formDialog({
    title: `Rename ${c.name}`,
    description: `${plural(c.symptoms, "symptom", "symptoms")} in this category keep their place.`,
    accept: "Save name",
    fields: [{ name: "name", label: "Name", required: true, value: c.name }],
    save: async (v) => { await api.editCategory(c.id, v.name); say(`${v.name} saved.`); },
  });
}

/** @param {any} c */
export async function deleteCategory(c) {
  const text = c.symptoms
    ? `${c.name} holds ${plural(c.symptoms, "symptom", "symptoms")}. They are kept, but become uncategorised.`
    : `${c.name} is empty. It is simply removed.`;
  if (!(await ask({ title: `Delete ${c.name}?`, text, accept: `Delete ${c.name}`, destructive: true }))) return false;
  try {
    await api.removeCategory(c.id);
    say(`${c.name} deleted.`);
    return true;
  } catch (e) {
    reportError(`Could not delete ${c.name}.`, e);
    return false;
  }
}
