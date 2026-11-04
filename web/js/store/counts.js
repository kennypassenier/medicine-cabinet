// The numbers shown next to symptoms, categories and forms, counted from the
// stored (and queue-overlaid) medicines and symptoms, so a link made offline
// counts at once and no number waits for a sync. Pure; same rule as the
// server's list queries (deleted medicines are not in the collection).

/** @param {Iterable<any>} keys @returns {Map<any, number>} */
function tally(keys) {
  const by = new Map();
  for (const k of keys) by.set(k, (by.get(k) ?? 0) + 1);
  return by;
}

/** Each symptom with how many medicines list it by name. @param {any[]} symptoms @param {any[]} medicines */
export function withMedicineCounts(symptoms, medicines) {
  const by = tally(medicines.flatMap((m) => [...new Set(/** @type {string[]} */ (m.symptoms ?? []))]));
  return symptoms.map((s) => ({ ...s, medicines: by.get(s.name) ?? 0 }));
}

/** Each form with how many medicines use it. @param {any[]} forms @param {any[]} medicines */
export function withFormCounts(forms, medicines) {
  const by = tally(medicines.filter((m) => m.form_id != null).map((m) => m.form_id));
  return forms.map((f) => ({ ...f, medicines: by.get(f.id) ?? 0 }));
}

/** Each category with how many symptoms belong to it. @param {any[]} categories @param {any[]} symptoms */
export function withSymptomCounts(categories, symptoms) {
  const by = tally(symptoms.filter((s) => s.category_id != null).map((s) => s.category_id));
  return categories.map((c) => ({ ...c, symptoms: by.get(c.id) ?? 0 }));
}
