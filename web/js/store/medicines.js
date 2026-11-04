// Medicines, Today, search and the dose guard, read from the local database
// with the plan module (the day plan, freshness and stock levels are worked out
// here with the browser clock). Writes go to the same routes as before.

import { ApiError } from "../api-error.js";
import { nowParam } from "../clock.js";
import { doseCheck, judge, overview, parseDate, serverOrder } from "../plan/index.js";
import { BASE, col, createLocal, dropWaiting, guard, send, waiting } from "./kit.js";
import { localIntakes } from "./intakes.js";
import { withQueue } from "./pending.js";

const missing = (/** @type {string} */ id) => new ApiError(`medicine ${id} does not exist`, "reload the list; it may have been deleted", 404);
const item = (/** @type {string} */ id) => `${BASE}/medicines/${id}`;

/** All medicines, judged for today, in the server's order. */
export async function localMedicines() {
  const [rows, forms] = await Promise.all([col("medicines").all(), col("forms").all()]);
  const formName = new Map(forms.map((/** @type {any} */ f) => [f.id, f.name]));
  const today = parseDate(nowParam());
  const laid = await withQueue(rows);
  return laid.map((m) => judge({ ...m, form: m.form_id == null ? "" : (formName.get(m.form_id) ?? m.form ?? "") }, today)).sort(serverOrder);
}

/** @param {string} id */
async function find(id) {
  const m = (await localMedicines()).find((x) => x.id === id);
  if (!m) throw missing(id);
  return m;
}

/** What the symptom search answers. @param {string} symptom */
async function search(symptom) {
  const needle = symptom.trim().toLowerCase();
  if (!needle) throw new ApiError("the symptom is needed", "fill in the symptom field", 400);
  const hits = (await localMedicines()).filter((m) => m.symptoms.some((/** @type {string} */ s) => s.includes(needle)));
  const usable = (/** @type {any} */ m) => m.active && m.stock_level !== "out" && m.freshness !== "expired" && !m.course_over;
  return { symptom: needle, usable: hits.filter(usable), unavailable: hits.filter((m) => !usable(m)) };
}

/** The body to send for an edit: what is stored, with the page's fields over it. @param {string} id @param {object} body */
async function merged(id, body) {
  const stored = await col("medicines").get(id);
  if (!stored) throw missing(id);
  const { pending: _queued, ...fields } = stored; // `pending` marks a record the server has not accepted yet; it is not part of the body
  return { stored, body: { ...fields, ...body, id } };
}

export const medicineStore = {
  medicines: () => guard(localMedicines),
  medicine: (/** @type {string} */ id) => guard(() => find(id)),
  today: () => guard(async () => overview(await localMedicines(), await localIntakes(), nowParam())),
  search: (/** @type {string} */ symptom) => guard(() => search(symptom)),
  doseCheck: (/** @type {string} */ id) => guard(async () => doseCheck(await find(id), await localIntakes(), nowParam())),
  create: (/** @type {object} */ body) => guard(() => createLocal("medicines", body)),
  update: (/** @type {string} */ id, /** @type {object} */ body) =>
    guard(async () => {
      const { stored, body: next } = await merged(id, body);
      await send("PUT", `${item(id)}?now=${nowParam()}`, next, stored.rev);
      return find(id);
    }),
  remove: (/** @type {string} */ id) =>
    guard(async () => {
      const stored = await col("medicines").get(id);
      if (!stored) throw missing(id);
      await send("DELETE", item(id), undefined, stored.rev);
      return { deleted: id };
    }),
  /** Bring back a medicine deleted in the last 14 days; a delete still waiting in the queue is simply taken back. */
  restore: (/** @type {string} */ id) =>
    guard(async () => {
      const waitingDelete = (await waiting()).some((e) => e.method === "DELETE" && e.path === item(id));
      if (waitingDelete && (await dropWaiting((e) => e.path === item(id)))) return find(id);
      return (await send("POST", `${item(id)}/restore?now=${nowParam()}`)).data;
    }),
  setActive: (/** @type {string} */ id, /** @type {boolean} */ active) =>
    guard(async () => {
      await send("POST", `${item(id)}/active?now=${nowParam()}`, { active });
      return find(id);
    }),
  addStock: (/** @type {string} */ id, /** @type {number} */ delta) =>
    guard(async () => {
      await send("POST", `${item(id)}/stock?now=${nowParam()}`, { delta });
      return find(id);
    }),
};
