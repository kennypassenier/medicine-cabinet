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

Update (2026-11-04): creating a medicine, symptom, category or form works offline.
The page makes the uid, the store creates through the kit's collection `create`
(queued POST with the kit's Idempotency-Key, shown at once by the overlay; a
medicine is shaped by `web/js/store/medicine-shape.js`). A dose, an edit or a
link made for such an item queues behind its create and stays valid because
every reference is a uid. A create the server refuses on replay (400, duplicate
name) lands in the kit's failed-edits list. The queue replays on the kit's
30 s retry timer after the connection returns.

Update (2026-11-04, links): links between a medicine and its symptoms made
offline show at once on Cabinet, Symptoms (search and counts) and survive a
reload; they reach the server with the queue. `store/link-effects.js` lays the
reverse mappings of waiting writes (a symptom created with `medicine_ids`, a
symptom rename or delete via the `was_name` hint, a category created with
`symptom_ids`, a form created with `medicine_ids`); a name typed in the medicine
form that no symptom has yet is created first as a queued symptom
(`store/symptom-ensure.js`). Undo of a symptom delete that is still queued takes
the delete back. A refused write makes every screen redraw (`local.js`).

Known limits: offline stock does not account for queued intake edits or deletes
beyond what stock-effects.js lays over; the reverse links are laid after the
kit's overlay, so a queued medicine edit that comes after an undo-created symptom
in the queue does not override it on screen (the server order wins after the
sync); undo of a deleted category or form is still a new create. There is no
screen that links medicines from the symptom side (only the undo of a delete).
Milestone 4 (hardening) is open: logout-guard wording, storage-cap message, a
recorded Playwright round trip.
