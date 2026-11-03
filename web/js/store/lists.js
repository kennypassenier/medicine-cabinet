// Symptoms, categories and forms: small lists the server keeps ordered by
// name. Adding needs a connection (the server hands out the id); editing and
// deleting queue. A delete answers `was`, the body that puts the item back,
// worked out here from the local data.

import { ApiError } from "../api-error.js";
import { BASE, col, createOnline, guard, nocase, numeric, send } from "./kit.js";
import { localMedicines } from "./medicines.js";

/** @param {string} name */
const route = (name) => `${BASE}/${name}`;

/** @param {string} name the collection */
async function sorted(name) {
  const rows = (await col(name).all()).map(numeric);
  return rows.sort((a, b) => nocase(a.name, b.name));
}

/** Symptoms, each with the category's current name. */
async function symptoms() {
  const [rows, cats] = await Promise.all([sorted("symptoms"), sorted("categories")]);
  const names = new Map(cats.map((c) => [c.id, c.name]));
  return rows.map((s) => ({ ...s, category: s.category_id == null ? null : (names.get(s.category_id) ?? s.category ?? null) }));
}

/** The stored item to edit, or the server's own "does not exist". @param {string} name @param {number} id @param {string} what */
async function stored(name, id, what) {
  const row = await col(name).get(String(id));
  if (!row) throw new ApiError(`${what} ${id} does not exist`, "reload the list; it may have been deleted", 404);
  return row;
}

/** PUT the item with the page's fields over the stored ones. @param {string} name @param {string} what @param {number} id @param {object} fields */
async function edit(name, what, id, fields) {
  const row = await stored(name, id, what);
  await send("PUT", `${route(name)}/${id}`, { ...row, ...fields, id: Number(id) }, row.rev);
  return { id: Number(id) };
}

/** @param {string} name @param {string} what @param {number} id @param {object} was */
async function remove(name, what, id, was) {
  const row = await stored(name, id, what);
  await send("DELETE", `${route(name)}/${id}`, undefined, row.rev);
  return { deleted: Number(id), was };
}

/** @param {number} id */
async function removeSymptom(id) {
  const row = await stored("symptoms", id, "symptom");
  const linked = (await localMedicines()).filter((m) => m.symptoms.includes(row.name)).map((m) => m.id);
  return remove("symptoms", "symptom", id, { category_id: row.category_id ?? null, medicine_ids: linked, name: row.name });
}

/** @param {number} id */
async function removeCategory(id) {
  const row = await stored("categories", id, "category");
  const members = (await sorted("symptoms")).filter((s) => s.category_id === Number(id)).map((s) => s.id);
  return remove("categories", "category", id, { name: row.name, symptom_ids: members });
}

/** @param {number} id */
async function removeForm(id) {
  const row = await stored("forms", id, "form");
  const users = (await localMedicines()).filter((m) => m.form_id === Number(id)).map((m) => m.id);
  return remove("forms", "form", id, { medicine_ids: users, name: row.name, unit: row.unit });
}

export const listStore = {
  symptoms: () => guard(symptoms),
  addSymptom: (/** @type {object} */ b) => guard(() => createOnline("A symptom", route("symptoms"), b)),
  editSymptom: (/** @type {number} */ id, /** @type {object} */ b) => guard(() => edit("symptoms", "symptom", id, b)),
  removeSymptom: (/** @type {number} */ id) => guard(() => removeSymptom(id)),
  categories: () => guard(() => sorted("categories")),
  addCategory: (/** @type {string} */ name, /** @type {number[]} */ symptomIds = []) => guard(() => createOnline("A category", route("categories"), { name, symptom_ids: symptomIds })),
  editCategory: (/** @type {number} */ id, /** @type {string} */ name) => guard(() => edit("categories", "category", id, { name })),
  removeCategory: (/** @type {number} */ id) => guard(() => removeCategory(id)),
  forms: () => guard(() => sorted("forms")),
  addForm: (/** @type {object} */ b) => guard(() => createOnline("A form", route("forms"), b)),
  editForm: (/** @type {number} */ id, /** @type {object} */ b) => guard(() => edit("forms", "form", id, b)),
  removeForm: (/** @type {number} */ id) => guard(() => removeForm(id)),
};
