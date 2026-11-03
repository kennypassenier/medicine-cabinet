// What waiting writes will do to a medicine once the server accepts them,
// shown now: stock goes down with a queued Take and moves with a queued
// "add stock", a queued switch on/off applies. Read from the kit's queue each
// time; the server's answer replaces it after the replay.

import { BASE, waiting } from "./kit.js";

const MEDICINE_POST = new RegExp(`^${BASE}/medicines/(\\d+)/(stock|active)$`);

/** @param {any} m @param {number} delta */
const moved = (m, delta) => ({ ...m, stock: Math.max(0, m.stock + delta) });

/** @param {Map<number, any>} by @param {any} e */
function apply(by, e) {
  const path = e.path.split("?")[0];
  const hit = MEDICINE_POST.exec(path);
  if (hit) {
    const m = by.get(Number(hit[1]));
    if (!m) return;
    by.set(m.id, hit[2] === "stock" ? moved(m, Number(e.body?.delta) || 0) : { ...m, active: e.body?.active === true });
  } else if (path === `${BASE}/intakes` && e.body?.status === "taken") {
    const m = by.get(Number(e.body.medicine_id));
    if (m) by.set(m.id, moved(m, -(Number(e.body.dose ?? 1) || 0)));
  }
}

/** The medicines with the waiting writes laid over them. @param {any[]} medicines numeric ids */
export async function withQueue(medicines) {
  const by = new Map(medicines.map((m) => [m.id, m]));
  for (const e of await waiting()) if (e.method === "POST") apply(by, e);
  return medicines.map((m) => by.get(m.id));
}
