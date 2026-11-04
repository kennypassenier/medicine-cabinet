import test from "node:test";
import assert from "node:assert/strict";
import { layMedicineLinks, laySymptomCategories } from "../../js/store/link-effects.js";
import { missingSymptoms } from "../../js/store/symptom-ensure.js";

const A = "aaaaaaaa-0000-4000-8000-000000000001";
const B = "aaaaaaaa-0000-4000-8000-000000000002";
const meds = () => [{ id: A, symptoms: ["fever"], form_id: null }, { id: B, symptoms: ["fever", "cough"], form_id: null }];
const post = (kind, body) => ({ method: "POST", path: `/api/mc/${kind}`, body });

test("a symptom created with medicine_ids lists itself on those medicines", () => {
  const out = layMedicineLinks(meds(), [post("symptoms", { id: "s1", name: " Wheeze ", medicine_ids: [B] })]);
  assert.deepEqual(out[0].symptoms, ["fever"]);
  assert.deepEqual(out[1].symptoms, ["cough", "fever", "wheeze"]);
});

test("a rename carries the old name's links over, a delete lets go of them", () => {
  const renamed = layMedicineLinks(meds(), [{ method: "PUT", path: "/api/mc/symptoms/s1", body: { name: "Pyrexia", was_name: "fever" } }]);
  assert.deepEqual(renamed.map((m) => m.symptoms), [["pyrexia"], ["cough", "pyrexia"]]);
  const gone = layMedicineLinks(meds(), [{ method: "DELETE", path: "/api/mc/symptoms/s1?was_name=fever" }]);
  assert.deepEqual(gone.map((m) => m.symptoms), [[], ["cough"]]);
});

test("an edit of the category only (same name) changes no links", () => {
  const out = layMedicineLinks(meds(), [{ method: "PUT", path: "/api/mc/symptoms/s1", body: { name: "fever", was_name: "fever", category_id: "c" } }]);
  assert.deepEqual(out, meds());
});

test("a form created with medicine_ids becomes the form of those medicines", () => {
  const out = layMedicineLinks(meds(), [post("forms", { id: "f1", name: "Syrup", medicine_ids: [A] })]);
  assert.deepEqual(out.map((m) => m.form_id), ["f1", null]);
});

test("a category created with symptom_ids files those symptoms; a deleted one lets go", () => {
  const syms = [{ id: "s1", category_id: null }, { id: "s2", category_id: "old" }];
  const filed = laySymptomCategories(syms, [post("categories", { id: "c1", name: "Air", symptom_ids: ["s1"] })]);
  assert.deepEqual(filed.map((s) => s.category_id), ["c1", "old"]);
  const freed = laySymptomCategories(syms, [{ method: "DELETE", path: "/api/mc/categories/old" }]);
  assert.deepEqual(freed.map((s) => s.category_id), [null, null]);
});

test("only the names the device does not hold are missing", () => {
  assert.deepEqual(missingSymptoms([" Cough ", "fever", "COUGH", ""], [{ name: "fever" }]), ["cough"]);
});
