// What the waiting writes do to the stock, worked out with the server's own
// rule (src/store/intakes.rs, src/store/stock.rs): a taken dose takes
// min(dose, stock); an edit gives back what that intake took and takes the new
// dose; a delete gives it back. Pure: entries in, medicines out, no kit.
//
// The served intake does not carry the server's `deducted`, so an edit or
// delete of an intake the queue did not create carries a hint of what the
// intake was (`was_dose` in the PUT body; `was_*` in the DELETE query), written
// by store/intakes.js. The hint assumes the intake took its whole dose.

export const BASE = "/api/mc";

const MEDICINE_POST = new RegExp(`^${BASE}/medicines/([^/]+)/(stock|active)$`);
const INTAKE_ITEM = new RegExp(`^${BASE}/intakes/([^/]+)$`);

/** @param {any} m @param {number} delta */
const moved = (m, delta) => ({ ...m, stock: Math.max(0, m.stock + delta) });

/** What a taken dose really takes. @param {number} stock @param {number} dose */
const takes = (stock, dose) => Math.max(0, Math.min(dose, stock));

/** @param {string} path */
function queryOf(path) {
  const q = path.split("?")[1];
  return new URLSearchParams(q ?? "");
}

/** @typedef {{ medicine: string, status: string, deducted: number }} Ledger */
/** @typedef {{ by: Map<string, any>, ledger: Map<string, Ledger> }} Laid */

/** Give back `old` and take `dose` on one medicine. @param {Laid} s @param {string} id @param {number} give @param {number} dose @returns {number} what was taken */
function restock(s, id, give, dose) {
  const m = s.by.get(id);
  if (!m) return 0;
  const back = moved(m, give);
  const took = takes(back.stock, dose);
  s.by.set(id, moved(back, -took));
  return took;
}

/** @param {Laid} s @param {any} body */
function created(s, body) {
  const taken = body?.status === "taken";
  const id = String(body?.medicine_id);
  const took = taken ? restock(s, id, 0, Number(body.dose ?? 1) || 0) : 0;
  if (body?.uid) s.ledger.set(String(body.uid), { medicine: id, status: String(body.status), deducted: took });
}

/** @param {Laid} s @param {string} uid @param {any} body */
function edited(s, uid, body) {
  const known = s.ledger.get(uid);
  const was = known ?? { medicine: String(body?.medicine_id), status: String(body?.status), deducted: Number(body?.was_dose ?? body?.dose) || 0 };
  const dose = Number(body?.dose);
  if (was.status !== "taken" || !Number.isFinite(dose)) return;
  s.ledger.set(uid, { ...was, deducted: restock(s, was.medicine, was.deducted, dose) });
}

/** @param {Laid} s @param {string} uid @param {string} path */
function deleted(s, uid, path) {
  const q = queryOf(path);
  const was = s.ledger.get(uid) ?? { medicine: q.get("was_medicine") ?? "", status: q.get("was_status") ?? "", deducted: Number(q.get("was_dose")) || 0 };
  if (was.status === "taken") restock(s, was.medicine, was.deducted, 0);
  s.ledger.delete(uid);
}

/** @param {Laid} s @param {any} e */
function apply(s, e) {
  const path = String(e.path).split("?")[0];
  const stockOrActive = MEDICINE_POST.exec(path);
  const item = INTAKE_ITEM.exec(path);
  if (e.method === "POST" && stockOrActive) {
    const m = s.by.get(stockOrActive[1]);
    if (m) s.by.set(m.id, stockOrActive[2] === "stock" ? moved(m, Number(e.body?.delta) || 0) : { ...m, active: e.body?.active === true });
  } else if (e.method === "POST" && path === `${BASE}/intakes`) created(s, e.body);
  else if (e.method === "PUT" && item) edited(s, item[1], e.body);
  else if (e.method === "DELETE" && item) deleted(s, item[1], String(e.path));
}

/**
 * The medicines with the waiting writes laid over them, in queue order.
 * @param {any[]} medicines uid ids @param {any[]} entries the kit's queue, oldest first
 */
export function layQueue(medicines, entries) {
  const s = { by: new Map(medicines.map((m) => [m.id, m])), ledger: new Map() };
  for (const e of entries) apply(s, e);
  return medicines.map((m) => s.by.get(m.id));
}
