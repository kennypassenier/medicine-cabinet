// A name typed into a medicine's "Helps with" field that no symptom has yet:
// the server makes the symptom when it takes the medicine, so a device that is
// offline makes it itself first (a queued create under its own uid), and the
// new symptom shows on the Symptoms page and in its counts at once. It queues
// ahead of the medicine write, so the replay finds the symptom already there.

import { col, createLocal, refresh } from "./kit.js";
import { cleanSymptoms } from "./medicine-shape.js";

/** The names in `wanted` that `known` (rows with a `name`) does not hold. Pure. @param {unknown} wanted @param {{ name: string }[]} known */
export function missingSymptoms(wanted, known) {
  const have = new Set(known.map((s) => s.name));
  return cleanSymptoms(wanted).filter((n) => !have.has(n));
}

/** Make the symptoms a medicine write names and the device does not hold. @param {unknown} names the body's `symptoms` */
export async function ensureSymptoms(names) {
  for (const name of missingSymptoms(names, await col("symptoms").all())) {
    try {
      await createLocal("symptoms", { name, category_id: null });
    } catch (e) {
      // Online with a stale copy the server may already hold it: refresh and carry on; anything else is a real failure.
      if (/** @type {any} */ (e)?.status !== 400) throw e;
      await refresh();
    }
  }
}
