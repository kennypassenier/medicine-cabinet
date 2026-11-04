// What waiting writes do to the links between the four kinds, shown now. The
// links live on the other side of the write: a symptom created with
// `medicine_ids` lists itself on those medicines, a rename or delete of a
// symptom changes the names the medicines carry, a category created with
// `symptom_ids` files those symptoms, a form created with `medicine_ids` is
// the form of those medicines. The kit's overlay only sees entries of its own
// collection, so these reverse mappings are laid here, over the medicines and
// symptoms it already overlaid. Pure: rows and queue entries in, rows out.
//
// A rename or delete does not carry the old name in the record, so the page
// writes it as a hint (`was_name` in the PUT body, in the DELETE query), the
// way store/intakes.js hints the old dose. The server ignores the hint.

import { cleanSymptoms } from "./medicine-shape.js";

const BASE = "/api/mc";

/** @param {string} path the path without its query @returns {{ kind: string, id: string | null }} */
function target(path) {
  const [kind, id] = path.slice(BASE.length + 1).split("/");
  return { kind, id: id ?? null };
}

/** @param {string} path */
const queryOf = (path) => new URLSearchParams(path.split("?")[1] ?? "");

/** Sorted, de-duplicated names, the way the server serves them. @param {string[]} names */
const sortedNames = (names) => [...new Set(names)].sort();

/** @param {any[]} rows @param {(m: any) => any} change @param {Set<string> | null} only the uids to touch, or null for all */
const mapRows = (rows, change, only = null) => rows.map((m) => (only === null || only.has(m.id) ? change(m) : m));

/** Each medicine listing `from` lists `to` instead (or nothing, with `to` null). @param {any[]} meds @param {string} from @param {string | null} to */
function renamed(meds, from, to) {
  return mapRows(meds, (m) => {
    if (!m.symptoms?.includes(from)) return m;
    const rest = m.symptoms.filter((/** @type {string} */ s) => s !== from);
    return { ...m, symptoms: sortedNames(to === null ? rest : [...rest, to]) };
  });
}

/** @param {any[]} meds @param {any} e */
function symptomEntry(meds, e) {
  const path = String(e.path);
  const { id } = target(path.split("?")[0]);
  if (e.method === "POST" && id === null) {
    const [name] = cleanSymptoms([e.body?.name]);
    const only = new Set(Array.isArray(e.body?.medicine_ids) ? e.body.medicine_ids.map(String) : []);
    return name && only.size ? mapRows(meds, (m) => ({ ...m, symptoms: sortedNames([...(m.symptoms ?? []), name]) }), only) : meds;
  }
  if (e.method === "PUT") {
    const [was] = cleanSymptoms([e.body?.was_name]);
    const [now] = cleanSymptoms([e.body?.name]);
    return was && now && was !== now ? renamed(meds, was, now) : meds;
  }
  if (e.method === "DELETE") {
    const [was] = cleanSymptoms([queryOf(path).get("was_name")]);
    return was ? renamed(meds, was, null) : meds;
  }
  return meds;
}

/** A form created with `medicine_ids` is the form of those medicines. @param {any[]} meds @param {any} e */
function formEntry(meds, e) {
  const only = new Set(Array.isArray(e.body?.medicine_ids) ? e.body.medicine_ids.map(String) : []);
  if (e.method !== "POST" || !e.body?.id || !only.size) return meds;
  return mapRows(meds, (m) => ({ ...m, form_id: e.body.id }), only);
}

/**
 * The medicines with the symptom and form links of the waiting writes laid over them.
 * @param {any[]} medicines @param {any[]} entries the kit's queue, oldest first, failed ones left out by the caller
 */
export function layMedicineLinks(medicines, entries) {
  return entries.reduce((meds, e) => {
    const { kind } = target(String(e.path).split("?")[0]);
    return kind === "symptoms" ? symptomEntry(meds, e) : kind === "forms" ? formEntry(meds, e) : meds;
  }, medicines);
}

/**
 * The symptoms with the waiting category writes laid over them: a category
 * created with `symptom_ids` files those symptoms, a deleted one lets go of its own.
 * @param {any[]} symptoms @param {any[]} entries
 */
export function laySymptomCategories(symptoms, entries) {
  return entries.reduce((rows, e) => {
    const { kind, id } = target(String(e.path).split("?")[0]);
    if (kind !== "categories") return rows;
    if (e.method === "POST" && e.body?.id && Array.isArray(e.body.symptom_ids)) {
      return mapRows(rows, (s) => ({ ...s, category_id: e.body.id, category: e.body.name ?? null }), new Set(e.body.symptom_ids.map(String)));
    }
    if (e.method === "DELETE" && id) return rows.map((s) => (s.category_id === id ? { ...s, category_id: null, category: null } : s));
    return rows;
  }, symptoms);
}
