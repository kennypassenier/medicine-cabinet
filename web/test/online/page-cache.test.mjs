import test from "node:test";
import assert from "node:assert/strict";
import { createPageCache, pageAt, PAGE_PARTS } from "../../js/store/page-cache.js";

/** A cache whose merged fetch is counted and answers every part of the page. */
function setup(initial = "today") {
  const state = { page: initial, calls: [], time: 0, fail: false };
  const cache = createPageCache({
    fetchPage: async (page) => {
      state.calls.push(page);
      if (state.fail) throw new Error("down");
      return Object.fromEntries(PAGE_PARTS[page].map((p) => [p, `${p}#${state.calls.length}`]));
    },
    currentPage: () => state.page,
    now: () => state.time,
    ttlMs: 100,
  });
  const single = (name) => () => Promise.resolve(`single:${name}`);
  return { state, cache, single };
}

test("paths map to their merged page", () => {
  assert.equal(pageAt("/"), "today");
  assert.equal(pageAt("/cabinet/"), "cabinet");
  assert.equal(pageAt("/history"), "history");
  assert.equal(pageAt("/medicine/new"), undefined);
});

test("one load of Today is one merged call for both resources", async () => {
  const { state, cache, single } = setup("today");
  const [a, b] = await Promise.all([cache.part("today", single("today")), cache.part("medicines", single("medicines"))]);
  assert.deepEqual(state.calls, ["today"]);
  assert.deepEqual([a, b], ["today#1", "medicines#1"]);
});

test("a part is handed out once: the next ask is a fresh merged call", async () => {
  const { state, cache, single } = setup("today");
  await Promise.all([cache.part("today", single("t")), cache.part("medicines", single("m"))]);
  await Promise.all([cache.part("today", single("t")), cache.part("medicines", single("m"))]);
  assert.deepEqual(state.calls, ["today", "today"]);
});

test("an expired answer is not reused", async () => {
  const { state, cache, single } = setup("cabinet");
  await cache.part("medicines", single("m"));
  state.time = 500;
  assert.equal(await cache.part("forms", single("f")), "forms#2");
  assert.equal(state.calls.length, 2);
});

test("a resource the page does not merge, or a pageless path, uses the single endpoint", async () => {
  const { state, cache, single } = setup("settings");
  assert.equal(await cache.part("medicines", single("medicines")), "single:medicines");
  state.page = undefined;
  assert.equal(await cache.part("forms", single("forms")), "single:forms");
  assert.deepEqual(state.calls, []);
});

test("an ask that is not the page's own (another limit) goes to the single endpoint", async () => {
  const { state, cache, single } = setup("history");
  assert.equal(await cache.part("intakes", single("intakes"), false), "single:intakes");
  assert.deepEqual(state.calls, []);
});

test("a write clears the answers", async () => {
  const { state, cache, single } = setup("cabinet");
  await cache.part("forms", single("f"));
  cache.clear();
  await cache.part("medicines", single("m"));
  assert.equal(state.calls.length, 2);
});

test("a failed merged call is not kept", async () => {
  const { state, cache, single } = setup("symptoms");
  state.fail = true;
  await assert.rejects(cache.part("symptoms", single("s")), /down/);
  state.fail = false;
  assert.equal(await cache.part("symptoms", single("s")), "symptoms#2");
});
