import test from "node:test";
import assert from "node:assert/strict";
import { listIntakes } from "../../js/store/intake-list.js";
import { layQueue } from "../../js/store/stock-effects.js";

const rows = [
  { id: "a", medicine_id: "m1", medicine_name: "old name", taken_at: "2026-11-04T08:00" },
  { id: "b", medicine_id: null, medicine_name: "Purged", taken_at: "2026-11-04T09:00" },
  { id: "c", medicine_id: "gone", medicine_name: "Gone", taken_at: "2026-11-03T09:00" },
];
const meds = [{ id: "m1", name: "Cetirizine" }];

test("an intake with a null medicine is kept with its own name", () => {
  const out = listIntakes(rows, meds, undefined, 50);
  assert.deepEqual(out.map((i) => [i.id, i.medicine_name]), [["b", "Purged"], ["a", "Cetirizine"], ["c", "Gone"]]);
});
test("the day filter and limit still apply", () => {
  assert.deepEqual(listIntakes(rows, meds, "2026-11-04", 50).map((i) => i.id), ["b", "a"]);
  assert.equal(listIntakes(rows, meds, undefined, 1).length, 1);
});
test("editing or deleting an intake without a medicine changes no stock", () => {
  const m = [{ id: "m1", stock: 8 }];
  const put = { method: "PUT", path: "/api/mc/intakes/b", body: { medicine_id: null, status: "taken", dose: 1, was_dose: 2 } };
  const del = { method: "DELETE", path: "/api/mc/intakes/b?was_medicine=null&was_status=taken&was_dose=2" };
  assert.equal(layQueue(m, [put, del])[0].stock, 8);
});
