# Offline plan (not built yet)

Kenny's standing rule (2026-11-02): a PWA keeps much of its data offline, in
its own browser database, and syncs with the server. The app does not build its
own store; it uses the kit's `window.chassisData` (chassis-rs 4.3.0,
`docs/OFFLINE_DESIGN.md` in that repo). Gaps go to `kp-gaps/chassis-rs.md`.

## What the kit gives

- `configure({ name, rev, routes })`, then `get/put/patch/delete` by path.
  A list route and an item route (`:id`) per resource; the copy is the last good
  GET answer per path, query string included in the key.
- Writes made without a connection are queued in order, replayed with
  `If-Match`; a 409 moves the edit to a failed list (the person reviews it).
- Server half: `IfMatch`, `Revisioned`, `Conflict` (u64 revisions, 409 body).

## What this app needs to fit it

1. **Revisions.** Every mutable table (medicines, symptoms, categories, forms,
   intakes) needs a `rev` column (migration V8) that rises on each write, and its
   routes answer `Revisioned` and check `IfMatch`.
2. **The clock in the URL.** Calls carry `now=` (browser local time), so every
   GET has a new cache key. Offline reads need keys without `now`: the data
   routes (medicines, intakes, symptoms, categories, forms) are fetched without
   it, and the time-dependent views are computed from them.
3. **Today offline.** `/api/mc/today` is computed on the server
   (`domain::plan`, `check_dose`). Offline, the page needs the same answer from
   cached medicines and intakes. Either the plan rules are ported to a pure JS
   module with the Rust tests as the spec, or the last `today` answer is kept as
   a read-only snapshot. This is the one real design choice; Kenny decides it
   with a form when the work starts.
4. **Dose logging offline.** Take/Skip are append-only intakes: they queue
   without conflicts. The stock deduction happens when the queue replays, so the
   dose guard for as-needed medicines must also run in the page offline.
5. **Undo** of a queued intake removes it from the queue instead of calling the
   server.

## Order

Wait for the chassis tag that carries the offline follow-ups on main (logout
guard on every page, band language, failed-edits view), adopt it, then build
1 to 5 in that order. No release before Kenny asks.
