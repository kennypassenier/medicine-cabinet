// Symptoms, categories and forms: small lists the server keeps ordered by
// name. Every write queues, adding too: the page makes the item's uid, so a new
// one shows (and can be linked) at once. A delete answers `was`, the body that
// puts the item back, worked out here from the local data.

import { ApiError } from "../api-error.js";
import { BASE, col, createLocal, guard, nocase, send } from "./kit.js";
import { withFormCounts, withMedicineCounts, withSymptomCounts } from "./counts.js";
import { localMedicines } from "./medicines.js";

/** @param {string} name */
const route = (name) => `${BASE}/${name}`;

/** @param {string} name the collection */
async function sorted(name) {
  const rows = await col(name).all();
  return rows.sort((a, b) => nocase(a.name, b.name));
}

/** The stored medicines (queue laid over them), for counting. */
const medicineRows = async () => col("medicines").all();

/** Symptoms, each with the category's current name and a locally counted medicine total. */
async function symptoms() {
  const [rows, cats, meds] = await Promise.all([sorted("symptoms"), sorted("categories"), medicineRows()]);
  const names = new Map(cats.map((c) => [c.id, c.name]));
  return withMedicineCounts(rows, meds).map((s) => ({ ...s, category: s.category_id == null ? null : (names.get(s.category_id) ?? s.category ?? null) }));
}

/** The stored item to edit, or the server's own "does not exist". @param {string} name @param {string} id @param {string} what */
async function stored(name, id, what) {
  const row = await col(name).get(id);
  if (!row) throw new ApiError(`${what} ${id} does not exist`, "reload the list; it may have been deleted", 404);
  return row;
}

/** PUT the item with the page's fields over the stored ones. @param {string} name @param {string} what @param {string} id @param {object} fields */
async function edit(name, what, id, fields) {
  const { pending: _queued, ...row } = await stored(name, id, what);
  await send("PUT", `${route(name)}/${id}`, { ...row, ...fields, id }, row.rev);
  return { id };
}

/** @param {string} name @param {string} what @param {string} id @param {object} was */
async function remove(name, what, id, was) {
  const row = await stored(name, id, what);
  await send("DELETE", `${route(name)}/${id}`, undefined, row.rev);
  return { deleted: id, was };
}

/** @param {string} id */
async function removeSymptom(id) {
  const row = await stored("symptoms", id, "symptom");
  const linked = (await localMedicines()).filter((m) => m.symptoms.includes(row.name)).map((m) => m.id);
  return remove("symptoms", "symptom", id, { category_id: row.category_id ?? null, medicine_ids: linked, name: row.name });
}

/** @param {string} id */
async function removeCategory(id) {
  const row = await stored("categories", id, "category");
  const members = (await sorted("symptoms")).filter((s) => s.category_id === id).map((s) => s.id);
  return remove("categories", "category", id, { name: row.name, symptom_ids: members });
}

/** @param {string} id */
async function removeForm(id) {
  const row = await stored("forms", id, "form");
  const users = (await localMedicines()).filter((m) => m.form_id === id).map((m) => m.id);
  return remove("forms", "form", id, { medicine_ids: users, name: row.name, unit: row.unit });
}

export const listStore = {
  symptoms: () => guard(symptoms),
  addSymptom: (/** @type {object} */ b) => guard(() => createLocal("symptoms", { category_id: null, ...b })),
  editSymptom: (/** @type {string} */ id, /** @type {object} */ b) => guard(() => edit("symptoms", "symptom", id, b)),
  removeSymptom: (/** @type {string} */ id) => guard(() => removeSymptom(id)),
  categories: () => guard(async () => withSymptomCounts(await sorted("categories"), await sorted("symptoms"))),
  addCategory: (/** @type {string} */ name, /** @type {string[]} */ symptomIds = []) => guard(() => createLocal("categories", { name, symptom_ids: symptomIds })),
  editCategory: (/** @type {string} */ id, /** @type {string} */ name) => guard(() => edit("categories", "category", id, { name })),
  removeCategory: (/** @type {string} */ id) => guard(() => removeCategory(id)),
  forms: () => guard(async () => withFormCounts(await sorted("forms"), await medicineRows())),
  addForm: (/** @type {object} */ b) => guard(() => createLocal("forms", b)),
  editForm: (/** @type {string} */ id, /** @type {object} */ b) => guard(() => edit("forms", "form", id, b)),
  removeForm: (/** @type {string} */ id) => guard(() => removeForm(id)),
};
