import test from "node:test";
import assert from "node:assert/strict";
import { cleanSymptoms, medicineFromBody, shapeMedicine } from "../../js/store/medicine-shape.js";

const UID = "dddddddd-4444-4444-8444-dddddddddddd";

test("symptoms are trimmed, lower-cased and de-duplicated like the server does", () => {
  assert.deepEqual(cleanSymptoms([" Cough ", "cough", "Sore Throat", ""]), ["cough", "sore throat"]);
  assert.deepEqual(cleanSymptoms(undefined), []);
});

test("a created medicine gets the server's defaults and keeps its uid", () => {
  const m = medicineFromBody({ id: UID, name: " Docs Test ", form_id: "f-1", stock: 10, slots: [{ time: "20:00", days: 31, dose: 2 }, { time: "08:00" }], expires_on: " " });
  assert.equal(m.id, UID);
  assert.equal(m.name, "Docs Test");
  assert.equal(m.form_id, "f-1");
  assert.equal(m.active, true);
  assert.equal(m.expires_on, null);
  assert.deepEqual(m.slots, [{ time: "08:00", days: 127, dose: 1 }, { time: "20:00", days: 31, dose: 2 }]);
});

test("the overlay function shapes a create and merges an edit", () => {
  const made = shapeMedicine(null, { method: "POST", body: { id: UID, name: "A", symptoms: ["X"] } });
  assert.deepEqual(made.symptoms, ["x"]);
  const edited = shapeMedicine(made, { method: "PUT", body: { name: "B", symptoms: ["Y", "y"] } });
  assert.equal(edited.name, "B");
  assert.deepEqual(edited.symptoms, ["y"]);
  assert.equal(edited.id, UID);
});
