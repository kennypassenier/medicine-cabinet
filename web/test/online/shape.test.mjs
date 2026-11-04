import test from "node:test";
import assert from "node:assert/strict";
import { created, intakeBody, intakeRows, loggedIntake, symptomBody } from "../../js/store/online-shape.js";
import { isOfflineFailure, localDatabaseProblem, probeIndexedDb } from "../../js/store/availability.js";

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

test("the local database is unusable without the kit, without indexedDB or when the probe fails", async () => {
  const idb = {};
  assert.match(await localDatabaseProblem({ kit: null, indexedDB: idb }), /not loaded/);
  assert.match(await localDatabaseProblem({ kit: {}, indexedDB: undefined }), /not available/);
  assert.match(await localDatabaseProblem({ kit: {}, indexedDB: idb }, async () => false), /cannot be opened/);
  assert.equal(await localDatabaseProblem({ kit: {}, indexedDB: idb }, async () => true), null);
});

test("the probe says no when open throws, errors or stays silent, and yes after a clean open", async () => {
  assert.equal(await probeIndexedDb(undefined), false);
  assert.equal(await probeIndexedDb({ open() { throw new Error("denied"); } }), false);
  assert.equal(await probeIndexedDb({ open() { const r = {}; setTimeout(() => r.onerror(), 0); return r; } }), false);
  assert.equal(await probeIndexedDb({ open() { return {}; } }, 20), false);
  let removed = false;
  const ok = { open() { const r = { result: { close() {} } }; setTimeout(() => r.onsuccess(), 0); return r; }, deleteDatabase() { removed = true; } };
  assert.equal(await probeIndexedDb(ok), true);
  assert.equal(removed, true);
});

test("only network failures count as offline, not a broken database", () => {
  assert.equal(isOfflineFailure({ offline: true }), true);
  assert.equal(isOfflineFailure({ status: 0 }), true);
  assert.equal(isOfflineFailure(new Error("InvalidStateError")), false);
});
