// Result shapes of the online-only store: the pages see what the local
// database would give them. Pure.

/** The intakes with the uid as `id`, like the local database serves them. @param {any[]} rows */
export const intakeRows = (rows) => rows.map((i) => ({ ...i, id: i.uid ?? i.id }));

/** The `{ id }` a create answered (the server adds the revision). @param {{ id: string }} r */
export const created = (r) => ({ id: r.id });

/** An intake write's `{ id }`: the uid, which the delete and edit routes take too. @param {{ id: number | string, uid?: string }} r */
export const loggedIntake = (r) => ({ id: r.uid ?? String(r.id) });

/** The POST body of a symptom put back: what `was` carried, without the old id (the server makes a new one). @param {any} was */
export function symptomBody(was) {
  const { was_id: _old, ...body } = was ?? {};
  return { category_id: null, ...body };
}

/** The body of a logged dose. @param {{ medicineId: string, status: string, plannedFor?: string | null, dose?: number, note?: string, takenAt: string }} i */
export function intakeBody(i) {
  return { medicine_id: i.medicineId, taken_at: i.takenAt, planned_for: i.plannedFor ?? null, status: i.status, dose: i.dose, note: i.note };
}
