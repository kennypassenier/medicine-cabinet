# Offline plan

Kenny's standing rule (2026-11-02): a PWA keeps much of its data offline in its
own browser database and syncs with the server; the server is always the master.
The app does not build its own store: it uses the kit's local database
(`chassisData.configure({ db: true })`, chassis-rs 4.5.0, `docs/LOCAL_DB_DESIGN.md`
in that repo). Gaps go to `~/Projects/kp-gaps/chassis-rs.md`.

Decided by Kenny (2026-11-03, form today-offline): **Vandaag is computed in the
page**, in JavaScript, from the local database. `domain/plan.rs` and
`check_dose` in `domain/schedule.rs` are the specification; the same cases go
into a JavaScript test.

## Shape

Five collections, each a table of the app database with a `rev` column and a
change-log entry written in the same transaction as every write
(`tx.sync_upsert` / `tx.sync_delete`, `use chassis::storage::sync::SyncWrite`):

| collection | key | body served |
|---|---|---|
| `medicines` | `id` (the uid, UUID text) | the medicine with its slots |
| `intakes` | `uid` (UUID text) | one logged dose |
| `symptoms` | `id` (uid) | symptom with its category and medicine links |
| `categories` | `id` (uid) | category |
| `forms` | `id` (uid) | form (tablet, drops, ...) |

Registered with `app.sync_collection(SyncCollection::new("medicines").key("id")
.rev("rev").routes("/api/mc/medicines", "/api/mc/medicines/:id"))` and one
`app.sync_load(...)` adapter that returns the current JSON body per key.

Intakes, medicines, symptoms, categories and forms are created on the device
(2026-11-04, Kenny): the page makes the uid with `crypto.randomUUID()`, posts it,
the server accepts a well-formed UUID it does not hold yet (`client_key`,
`check_create`). The integer id is internal only (migration V10 added `uid`). Editing and deleting existing ones queue with `If-Match`.

## Milestones (no release before Kenny asks)

1. **Server feed.** Migration V8 (`rev` on the five tables, `uid` on intakes,
   backfilled), every store write bumps `rev` and records the change, the
   five collections and the load adapter, `IfMatch` / `Revisioned` on the write
   routes, intake create with a client `uid` and an idempotency check.
2. **Plan in JavaScript.** `web/js/plan/` : a pure module with the day plan and
   the dose guard, plus a JavaScript test over the same cases as the Rust tests
   (`node --test`, run only at a release).
3. **Page on the local database.** `api.js` reads the five collections through
   `chassisData.db.collection(...)`; Today, Cabinet, Symptoms, History and the
   dose guard use them; Take / Skip / Undo go through the queue; the connection
   band and the failed-edits drawer come from the kit.
4. **Hardening.** Logout guard text, storage cap message, a Playwright
   check of the offline round trip (Firefox), kp-gaps entries for what the kit
   lacks.

## What stays on the server

Reminders, the Almanac sync, stock deduction (it happens when the intake write
is accepted, as today), the strict date and amount validation. A write refused
on replay goes to the kit's failed list and the server's version wins.

## Status (2026-11-03)

Milestones 1 to 3 are built and merged on `adopt-chassis-next`. Checked in
Firefox against a built binary: Today, Cabinet, Symptoms and History render
after a full page reload with the network off; a Take made offline reaches the
server once the connection returns and the stock drops once; undo of a queued
Take empties the queue; an offline edit the server refuses (duplicate form
name, 400) is shown by the kit's notice and the server's version wins.

Known limits: creating a medicine, symptom, category or form needs a
connection (the server hands out the id); offline stock does not account for
queued intake edits or deletes; symptom, category and form counts stay stale
until the next sync. Milestone 4 (hardening) is open: logout-guard wording,
storage-cap message, a recorded Playwright round trip.
