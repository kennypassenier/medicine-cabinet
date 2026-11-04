import test from "node:test";
import assert from "node:assert/strict";
import { created, intakeBody, intakeRows, loggedIntake, symptomBody } from "../../js/store/online-shape.js";
import { isOfflineFailure, localDatabaseProblem } from "../../js/store/availability.js";

test("intakes are keyed by their uid, like the local database", () => {
  assert.deepEqual(intakeRows([{ id: 7, uid: "u-7", note: "" }, { id: 8 }]).map((i) => i.id), ["u-7", 8]);
});

test("a create answers only its id; a logged dose answers its uid", () => {
  assert.deepEqual(created({ id: "x", rev: 3 }), { id: "x" });
  assert.deepEqual(loggedIntake({ id: 7, uid: "u-7" }), { id: "u-7" });
  assert.deepEqual(loggedIntake({ id: 7 }), { id: "7" });
});

test("a symptom put back loses its old id and keeps its links", () => {
  const body = symptomBody({ name: "cough", medicine_ids: ["m1"], was_id: "old" });
  assert.deepEqual(body, { category_id: null, name: "cough", medicine_ids: ["m1"] });
});

test("the intake body uses the API's field names", () => {
  const body = intakeBody({ medicineId: "m1", status: "taken", takenAt: "2026-11-04T08:00", dose: 2 });
  assert.deepEqual(body, { medicine_id: "m1", taken_at: "2026-11-04T08:00", planned_for: null, status: "taken", dose: 2, note: undefined });
});

test("the local database is unusable without the kit or when the kit fell back to memory", async () => {
  assert.match(await localDatabaseProblem(null), /not loaded/);
  assert.match(await localDatabaseProblem({ db: { persistent: Promise.resolve(false) } }), /memory/);
  assert.equal(await localDatabaseProblem({ db: { persistent: Promise.resolve(true) } }), null);
});

test("an offline or silent kit cannot tell, a failing one is a problem", async () => {
  assert.equal(await localDatabaseProblem({ db: { persistent: Promise.reject({ offline: true }) } }), null);
  assert.equal(await localDatabaseProblem({ db: { persistent: new Promise(() => {}) } }, 20), null);
  assert.match(await localDatabaseProblem({ db: { persistent: Promise.reject(new Error("boom")) } }), /boom/);
});

test("only network failures count as offline, not a broken database", () => {
  assert.equal(isOfflineFailure({ offline: true }), true);
  assert.equal(isOfflineFailure({ status: 0 }), true);
  assert.equal(isOfflineFailure(new Error("InvalidStateError")), false);
});
