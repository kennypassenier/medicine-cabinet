import test from "node:test";
import assert from "node:assert/strict";
import { layQueue } from "../../js/store/stock-effects.js";

const BASE = "/api/mc";
const med = (stock) => [{ id: "m1", stock, active: true }];
const take = (uid, dose) => ({ method: "POST", path: `${BASE}/intakes`, body: { uid, medicine_id: "m1", status: "taken", dose } });
const put = (uid, dose, was) => ({ method: "PUT", path: `${BASE}/intakes/${uid}`, body: { uid, medicine_id: "m1", status: "taken", dose, was_dose: was } });
const del = (uid, dose = 2) => ({ method: "DELETE", path: `${BASE}/intakes/${uid}?was_medicine=m1&was_status=taken&was_dose=${dose}` });
const stockOf = (entries, stock = 10) => layQueue(med(stock), entries)[0].stock;

test("a queued take lowers the stock", () => assert.equal(stockOf([take("a", 2)]), 8));
test("a take never goes below zero", () => assert.equal(stockOf([take("a", 5)], 3), 0));
test("a skip changes nothing", () => {
  const skip = { method: "POST", path: `${BASE}/intakes`, body: { uid: "s", medicine_id: "m1", status: "skipped", dose: 2 } };
  assert.equal(stockOf([skip]), 10);
});

test("editing a queued take: 10, take 2, edit to 1 -> 9", () => assert.equal(stockOf([take("a", 2), put("a", 1, 2)]), 9));
test("deleting a queued take gives it back", () => assert.equal(stockOf([take("a", 2), del("a")]), 10));
test("edit then delete of a queued take", () => assert.equal(stockOf([take("a", 2), put("a", 1, 2), del("a", 2)]), 10));
test("two edits in a row follow the last dose", () => assert.equal(stockOf([take("a", 2), put("a", 1, 2), put("a", 3, 2)]), 7));

// 10 in the server's stock already includes a synced take of 2 (stock was 12).
test("editing a synced intake uses the hint: 8 on the server, 2 -> 1 gives 9", () => assert.equal(stockOf([put("b", 1, 2)], 8), 9));
test("deleting a synced intake gives its dose back", () => assert.equal(stockOf([del("b", 2)], 8), 10));
test("edit then delete of a synced intake", () => assert.equal(stockOf([put("b", 1, 2), del("b", 2)], 8), 10));

test("a raised dose is limited by the stock like the server", () => { assert.equal(stockOf([put("b", 9, 2)], 8), 1); assert.equal(stockOf([put("b", 20, 2)], 8), 0); });
test("a skipped intake edited or deleted leaves the stock", () => {
  const e = { method: "PUT", path: `${BASE}/intakes/c`, body: { medicine_id: "m1", status: "skipped", dose: 3, was_dose: 1 } };
  const d = { method: "DELETE", path: `${BASE}/intakes/c?was_medicine=m1&was_status=skipped&was_dose=1` };
  assert.equal(stockOf([e, d]), 10);
});
test("an edit of an unknown intake without a hint changes nothing", () => {
  const e = { method: "PUT", path: `${BASE}/intakes/z`, body: { medicine_id: "m1", status: "taken", dose: 4 } };
  assert.equal(stockOf([e], 8), 8);
});
test("add stock and switch still apply, other medicines are untouched", () => {
  const out = layQueue([{ id: "m1", stock: 5, active: true }, { id: "m2", stock: 7, active: true }], [
    { method: "POST", path: `${BASE}/medicines/m1/stock?now=x`, body: { delta: 3 } },
    { method: "POST", path: `${BASE}/medicines/m1/active`, body: { active: false } },
  ]);
  assert.deepEqual(out, [{ id: "m1", stock: 8, active: false }, { id: "m2", stock: 7, active: true }]);
});

test("a medicine made offline takes its stock from a queued take (uid with dashes)", () => {
  const uid = "dddddddd-4444-4444-8444-dddddddddddd";
  const out = layQueue([{ id: uid, stock: 10, active: true }], [
    { method: "POST", path: `${BASE}/intakes`, body: { uid: "t", medicine_id: uid, status: "taken", dose: 2 } },
    { method: "POST", path: `${BASE}/medicines/${uid}/stock?now=x`, body: { delta: 1 } },
  ]);
  assert.equal(out[0].stock, 9);
});
