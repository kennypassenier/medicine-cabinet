// What waiting writes will do to a medicine once the server accepts them,
// shown now: stock moves with a queued Take, an edit or delete of an intake and
// "add stock", a queued switch on/off applies. Read from the kit's queue each
// time; the server's answer replaces it after the replay. The rules live in
// stock-effects.js (stock, active) and link-effects.js (symptom and form links), both pure.

import { waiting } from "./kit.js";
import { layMedicineLinks } from "./link-effects.js";
import { layQueue } from "./stock-effects.js";

/** The medicines with the waiting writes laid over them. @param {any[]} medicines uid ids */
export async function withQueue(medicines) {
  const entries = await waiting();
  return layMedicineLinks(layQueue(medicines, entries), entries);
}
