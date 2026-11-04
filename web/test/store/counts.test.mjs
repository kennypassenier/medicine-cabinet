import test from "node:test";
import assert from "node:assert/strict";
import { withFormCounts, withMedicineCounts, withSymptomCounts } from "../../js/store/counts.js";

const meds = [
  { id: "m1", symptoms: ["headache", "fever"], form_id: "f1" },
  { id: "m2", symptoms: ["headache"], form_id: "f1" },
  { id: "m3", symptoms: [], form_id: null },
];

test("symptoms count the medicines that list them", () => {
  const out = withMedicineCounts([{ name: "headache" }, { name: "fever" }, { name: "cough" }], meds);
  assert.deepEqual(out.map((s) => s.medicines), [2, 1, 0]);
});
test("a link made offline counts at once", () => {
  const linked = [...meds, { id: "m4", symptoms: ["cough", "cough"], form_id: "f2" }];
  assert.equal(withMedicineCounts([{ name: "cough" }], linked)[0].medicines, 1);
});
test("forms count their medicines by uid", () => {
  assert.deepEqual(withFormCounts([{ id: "f1" }, { id: "f2" }], meds).map((f) => f.medicines), [2, 0]);
});
test("categories count their symptoms", () => {
  const out = withSymptomCounts([{ id: "c1" }, { id: "c2" }], [{ category_id: "c1" }, { category_id: "c1" }, { category_id: null }]);
  assert.deepEqual(out.map((c) => c.symptoms), [2, 0]);
});
