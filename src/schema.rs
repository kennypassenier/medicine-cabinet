//! The app database schema, as numbered migrations (applied once, in order).

use chassis::storage::Migration;

/// SQL that makes a version-4-shaped UUID (36 characters) from `randomblob`:
/// used to backfill intakes and to give a server-made intake its `uid`.
macro_rules! uid_sql {
    () => {
        "lower(hex(randomblob(4))) || '-' || lower(hex(randomblob(2))) || '-4' \
         || substr(lower(hex(randomblob(2))), 2) || '-' \
         || substr('89ab', 1 + (abs(random()) % 4), 1) \
         || substr(lower(hex(randomblob(2))), 2) || '-' || lower(hex(randomblob(6)))"
    };
}

/// [`uid_sql!`] as a value, for statements built at run time.
pub const UID_SQL: &str = uid_sql!();

const V1: &str = "
CREATE TABLE medicines (
    id               INTEGER PRIMARY KEY,
    name             TEXT NOT NULL,
    purpose          TEXT NOT NULL DEFAULT '',
    form             TEXT NOT NULL DEFAULT '',
    strength         TEXT NOT NULL DEFAULT '',
    notes            TEXT NOT NULL DEFAULT '',
    active           INTEGER NOT NULL DEFAULT 1,
    as_needed        INTEGER NOT NULL DEFAULT 0,
    min_gap_minutes  INTEGER,
    max_per_day      INTEGER,
    stock            REAL NOT NULL DEFAULT 0,
    stock_unit       TEXT NOT NULL DEFAULT '',
    low_stock_at     REAL,
    expires_on       TEXT,
    opened_on        TEXT,
    shelf_life_days  INTEGER,
    course_ends_on   TEXT,
    created_at       TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE symptoms (
    id    INTEGER PRIMARY KEY,
    name  TEXT NOT NULL UNIQUE
);
CREATE TABLE medicine_symptoms (
    medicine_id INTEGER NOT NULL REFERENCES medicines(id) ON DELETE CASCADE,
    symptom_id  INTEGER NOT NULL REFERENCES symptoms(id) ON DELETE CASCADE,
    PRIMARY KEY (medicine_id, symptom_id)
);
CREATE TABLE schedule_slots (
    id          INTEGER PRIMARY KEY,
    medicine_id INTEGER NOT NULL REFERENCES medicines(id) ON DELETE CASCADE,
    time_of_day TEXT NOT NULL,
    days        INTEGER NOT NULL DEFAULT 127,
    dose        REAL NOT NULL DEFAULT 1
);
CREATE TABLE intakes (
    id          INTEGER PRIMARY KEY,
    medicine_id INTEGER NOT NULL REFERENCES medicines(id) ON DELETE CASCADE,
    -- Local wall-clock times as the browser sent them: 'YYYY-MM-DDTHH:MM'.
    taken_at    TEXT NOT NULL,
    planned_for TEXT,
    status      TEXT NOT NULL CHECK (status IN ('taken','skipped')),
    dose        REAL NOT NULL DEFAULT 1,
    note        TEXT NOT NULL DEFAULT ''
);
CREATE INDEX intakes_by_medicine ON intakes (medicine_id, taken_at);
CREATE INDEX slots_by_medicine ON schedule_slots (medicine_id);
";

/// Symptoms become first-class: they get optional categories and are no
/// longer deleted when the last medicine stops using them.
const V2: &str = "
CREATE TABLE symptom_categories (
    id    INTEGER PRIMARY KEY,
    name  TEXT NOT NULL UNIQUE COLLATE NOCASE
);
ALTER TABLE symptoms ADD COLUMN category_id INTEGER
    REFERENCES symptom_categories(id) ON DELETE SET NULL;
";

/// Medicine forms (tablet, syrup, ...) become a managed list with the unit
/// stock is counted in. The old free-text `medicines.form` column is kept
/// (SQLite cannot drop it cheaply) but no longer read; its values are
/// carried over into the new list.
const V3: &str = "
CREATE TABLE medicine_forms (
    id    INTEGER PRIMARY KEY,
    name  TEXT NOT NULL UNIQUE COLLATE NOCASE,
    unit  TEXT NOT NULL DEFAULT ''
);
INSERT INTO medicine_forms (name, unit) VALUES
    ('Tablet', 'tablets'), ('Capsule', 'capsules'), ('Syrup', 'ml'),
    ('Drops', 'drops'), ('Cream', 'g'), ('Spray', 'doses'),
    ('Inhaler', 'puffs'), ('Patch', 'patches'), ('Injection', 'doses');
INSERT OR IGNORE INTO medicine_forms (name) SELECT DISTINCT form FROM medicines WHERE form <> '';
ALTER TABLE medicines ADD COLUMN form_id INTEGER
    REFERENCES medicine_forms(id) ON DELETE SET NULL;
UPDATE medicines SET form_id = (SELECT id FROM medicine_forms WHERE name = medicines.form)
    WHERE form <> '';
";

/// Which reminders were already sent, so a restart never repeats one.
/// `planned_for` is the slot (`YYYY-MM-DDTHH:MM`) or, for the daily
/// attention digest, the date.
const V4: &str = "
CREATE TABLE reminder_log (
    medicine_id INTEGER NOT NULL REFERENCES medicines(id) ON DELETE CASCADE,
    planned_for TEXT NOT NULL,
    kind        TEXT NOT NULL,
    sent_at     TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (medicine_id, planned_for, kind)
);
";

/// What was last sent to the calendar hub, per external id, so only changes
/// are posted and an event that is no longer wanted can be deleted.
const V5: &str = "
CREATE TABLE agenda_sent (
    external_id TEXT PRIMARY KEY,
    fingerprint TEXT NOT NULL,
    sent_at     TEXT NOT NULL DEFAULT (datetime('now'))
);
";

/// Deleting a medicine is soft for two weeks, so the app can offer an undo.
const V6: &str = "
ALTER TABLE medicines ADD COLUMN deleted_at TEXT;
";

/// How much of a taken dose actually left the stock (it cannot go below
/// zero), so undoing or editing the intake puts back exactly that much.
/// Rows from before this column keep NULL and fall back to their dose.
const V7: &str = "
ALTER TABLE intakes ADD COLUMN deducted REAL;
";

/// Offline sync: every synced row carries a revision (starts at 1, bumped by
/// each write), and an intake gets a stable `uid` (UUID text) the browser can
/// make while offline. `idempotency_key` remembers the `Idempotency-Key` an
/// intake was created with, so a replayed request is answered, not repeated.
/// Existing intakes get a version-4-shaped `uid` from `randomblob`.
const V8: &str = concat!(
    "
ALTER TABLE medicines ADD COLUMN rev INTEGER NOT NULL DEFAULT 1;
ALTER TABLE symptoms ADD COLUMN rev INTEGER NOT NULL DEFAULT 1;
ALTER TABLE symptom_categories ADD COLUMN rev INTEGER NOT NULL DEFAULT 1;
ALTER TABLE medicine_forms ADD COLUMN rev INTEGER NOT NULL DEFAULT 1;
ALTER TABLE intakes ADD COLUMN rev INTEGER NOT NULL DEFAULT 1;
ALTER TABLE intakes ADD COLUMN uid TEXT;
ALTER TABLE intakes ADD COLUMN idempotency_key TEXT;
UPDATE intakes SET uid = ",
    uid_sql!(),
    ";
CREATE UNIQUE INDEX intakes_by_uid ON intakes (uid);
"
);

pub fn migrations() -> Vec<Migration> {
    vec![
        Migration::sql(V1),
        Migration::sql(V2),
        Migration::sql(V3),
        Migration::sql(V4),
        Migration::sql(V5),
        Migration::sql(V6),
        Migration::sql(V7),
        Migration::sql(V8),
    ]
}

/// Every migration's SQL, in order, for tests that run the stores against an
/// in-memory database.
#[cfg(test)]
pub const ALL_SQL: [&str; 8] = [V1, V2, V3, V4, V5, V6, V7, V8];
