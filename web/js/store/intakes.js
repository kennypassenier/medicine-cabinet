// The intake log. An intake is keyed by the `uid` the page makes while it may
// be offline; the pages see that uid as the intake's `id`. A Take / Skip is
// shown from the queue at once (pending), the server deducts stock when it
// accepts the write.

import { ApiError } from "../api-error.js";
import { nowParam } from "../clock.js";
import { BASE, col, dropWaiting, guard, send } from "./kit.js";

const LIST = `${BASE}/intakes`;
const missing = new ApiError("that intake does not exist", "reload the list; it may have been deleted", 404);

/** @param {any} i */
const named = (i) => ({ ...i, id: i.uid });

/** Every stored intake, pages' shape (`id` is the uid). */
export async function localIntakes() {
  return (await col("intakes").all()).map(named);
}

/** @param {number} id the medicine id */
async function medicineName(id) {
  const m = await col("medicines").get(String(id));
  if (!m) throw new ApiError(`medicine ${id} does not exist`, "reload the list; it may have been deleted", 404);
  return m.name;
}

/** Newest first, for a day (or month) prefix. @param {string | undefined} day @param {number} limit */
async function list(day, limit) {
  const [rows, medicines] = await Promise.all([localIntakes(), col("medicines").all()]);
  const names = new Map(medicines.map((/** @type {any} */ m) => [Number(m.id), m.name]));
  return rows
    .filter((i) => names.has(i.medicine_id) && (!day || i.taken_at.startsWith(day)))
    .map((i) => ({ ...i, medicine_name: names.get(i.medicine_id) }))
    .sort((a, b) => (a.taken_at < b.taken_at ? 1 : a.taken_at > b.taken_at ? -1 : 0))
    .slice(0, Math.min(1000, Math.max(1, limit)));
}

/** @param {{ medicineId: number, status: "taken" | "skipped", plannedFor?: string | null, dose?: number, note?: string, takenAt?: string }} i */
async function log(i) {
  const uid = crypto.randomUUID();
  const body = {
    uid, medicine_id: i.medicineId, medicine_name: await medicineName(i.medicineId), taken_at: i.takenAt ?? nowParam(),
    planned_for: i.plannedFor ?? null, status: i.status, dose: i.dose ?? 1, note: i.note ?? "",
  };
  await send("POST", LIST, body);
  return { id: uid };
}

/** Undo: an intake still in the queue is taken back, otherwise the delete is sent (or queued). @param {string} uid */
async function undo(uid) {
  const own = `${LIST}/${uid}`;
  const unsent = await dropWaiting((e) => e.method === "POST" && e.path === LIST && e.body?.uid === uid);
  if (unsent) await dropWaiting((e) => e.path === own); // edits queued behind it have nothing left to change
  else await send("DELETE", own);
  return { undone: uid };
}

/** @param {string} uid @param {{ taken_at: string, dose: number, note: string }} edit */
async function edit(uid, edit) {
  const stored = await col("intakes").get(uid);
  if (!stored) throw missing;
  await send("PUT", `${LIST}/${uid}`, { ...stored, ...edit }, stored.rev);
  return { id: uid };
}

export const intakeStore = {
  intakes: (/** @type {string | undefined} */ day, limit = 200) => guard(() => list(day, limit)),
  logIntake: (/** @type {Parameters<typeof log>[0]} */ i) => guard(() => log(i)),
  undoIntake: (/** @type {string} */ uid) => guard(() => undo(String(uid))),
  editIntake: (/** @type {string} */ uid, /** @type {any} */ b) => guard(() => edit(String(uid), b)),
};
