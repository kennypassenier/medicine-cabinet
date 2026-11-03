// Medicines, Today, search and the dose guard, read from the local database
// with the plan module (the day plan, freshness and stock levels are worked out
// here with the browser clock). Writes go to the same routes as before.

import { ApiError } from "../api-error.js";
import { nowParam } from "../clock.js";
import { doseCheck, judge, overview, parseDate, serverOrder } from "../plan/index.js";
import { BASE, col, createOnline, dropWaiting, guard, numeric, send, waiting } from "./kit.js";
import { localIntakes } from "./intakes.js";
import { withQueue } from "./pending.js";

const missing = (/** @type {number} */ id) => new ApiError(`medicine ${id} does not exist`, "reload the list; it may have been deleted", 404);
const item = (/** @type {number} */ id) => `${BASE}/medicines/${id}`;

/** All medicines, judged for today, in the server's order. */
export async function localMedicines() {
  const [rows, forms] = await Promise.all([col("medicines").all(), col("forms").all()]);
  const formName = new Map(forms.map((/** @type {any} */ f) => [Number(f.id), f.name]));
  const today = parseDate(nowParam());
  const laid = await withQueue(rows.map(numeric));
  return laid.map((m) => judge({ ...m, form: m.form_id == null ? "" : (formName.get(m.form_id) ?? m.form ?? "") }, today)).sort(serverOrder);
}

/** @param {number} id */
async function find(id) {
  const m = (await localMedicines()).find((x) => x.id === Number(id));
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

/** The body to send for an edit: what is stored, with the page's fields over it. @param {number} id @param {object} body */
async function merged(id, body) {
  const stored = await col("medicines").get(String(id));
  if (!stored) throw missing(id);
  return { stored, body: { ...stored, ...body, id: Number(id) } };
}

export const medicineStore = {
  medicines: () => guard(localMedicines),
  medicine: (/** @type {number} */ id) => guard(() => find(id)),
  today: () => guard(async () => overview(await localMedicines(), await localIntakes(), nowParam())),
  search: (/** @type {string} */ symptom) => guard(() => search(symptom)),
  doseCheck: (/** @type {number} */ id) => guard(async () => doseCheck(await find(id), await localIntakes(), nowParam())),
  create: (/** @type {object} */ body) => guard(() => createOnline("A medicine", `${BASE}/medicines?now=${nowParam()}`, body)),
  update: (/** @type {number} */ id, /** @type {object} */ body) =>
    guard(async () => {
      const { stored, body: next } = await merged(id, body);
      await send("PUT", `${item(id)}?now=${nowParam()}`, next, stored.rev);
      return find(id);
    }),
  remove: (/** @type {number} */ id) =>
    guard(async () => {
      const stored = await col("medicines").get(String(id));
      if (!stored) throw missing(id);
      await send("DELETE", item(id), undefined, stored.rev);
      return { deleted: Number(id) };
    }),
  /** Bring back a medicine deleted in the last 14 days; a delete still waiting in the queue is simply taken back. */
  restore: (/** @type {number} */ id) =>
    guard(async () => {
      const waitingDelete = (await waiting()).some((e) => e.method === "DELETE" && e.path === item(id));
      if (waitingDelete && (await dropWaiting((e) => e.path === item(id)))) return find(id);
      return (await send("POST", `${item(id)}/restore?now=${nowParam()}`)).data;
    }),
  setActive: (/** @type {number} */ id, /** @type {boolean} */ active) =>
    guard(async () => {
      await send("POST", `${item(id)}/active?now=${nowParam()}`, { active });
      return find(id);
    }),
  addStock: (/** @type {number} */ id, /** @type {number} */ delta) =>
    guard(async () => {
      await send("POST", `${item(id)}/stock?now=${nowParam()}`, { delta });
      return find(id);
    }),
};
