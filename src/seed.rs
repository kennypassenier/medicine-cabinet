//! Demo data for development builds only.
//!
//! The whole module is compiled out of release builds (`debug_assertions`
//! off), so a released binary can never put demo medicines into a real
//! cabinet. In a debug build the seed runs once, when the cabinet is empty.

use std::time::Duration;

use chassis::Error;
use chassis::storage::AppDb;
use chassis::storage::rusqlite::{Transaction, params};

use crate::domain::date::Date;
use crate::domain::medicine::MedicineInput;
use crate::domain::schedule::SlotInput;
use crate::store::{categories, medicines, slots};

/// Seed in the background: the database opens when the service starts, so
/// retry until it does.
pub fn spawn(db: AppDb) {
    tokio::spawn(async move {
        for _ in 0..100 {
            tokio::time::sleep(Duration::from_millis(200)).await;
            match db.write(seed_if_empty).await {
                Ok(()) => return,
                Err(e) if e.message.contains("not open yet") => continue,
                Err(e) => return eprintln!("demo data was not added: {e}"),
            }
        }
    });
}

fn today() -> Date {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    Date::from_days((secs / 86_400) as i64)
}

fn seed_if_empty(tx: &Transaction) -> Result<(), Error> {
    let count: i64 = tx
        .query_row("SELECT COUNT(*) FROM medicines", [], |r| r.get(0))
        .map_err(crate::store::db_err)?;
    if count > 0 {
        return Ok(());
    }
    let ids = seed_categories(tx)?;
    for (input, day_slots) in demo_medicines(today()) {
        let id = medicines::insert(tx, &input)?;
        set_form(tx, id, form_of(&input.name))?;
        slots::replace(tx, id, &day_slots)?;
    }
    categorise(tx, &ids)
}

/// The demo medicines' forms, by name.
fn form_of(medicine: &str) -> &'static str {
    match medicine {
        "Amoxicillin" | "Omeprazole" | "Vitamin D" => "Capsule",
        _ => "Tablet",
    }
}

fn set_form(tx: &Transaction, medicine_id: i64, form: &str) -> Result<(), Error> {
    tx.execute(
        "UPDATE medicines SET form_id = (SELECT id FROM medicine_forms WHERE name = ?2) WHERE id = ?1",
        params![medicine_id, form],
    )
    .map_err(crate::store::db_err)?;
    Ok(())
}

fn seed_categories(tx: &Transaction) -> Result<Vec<(&'static str, i64)>, Error> {
    [
        "Head",
        "Stomach",
        "Muscles",
        "Throat & nose",
        "Eyes",
        "Sleep & energy",
        "General",
    ]
    .into_iter()
    .map(|name| Ok((name, categories::create(tx, name, &[])?)))
    .collect()
}

/// Which category each demo symptom belongs to.
fn categorise(tx: &Transaction, ids: &[(&str, i64)]) -> Result<(), Error> {
    const LINKS: [(&str, &str); 10] = [
        ("headache", "Head"),
        ("toothache", "Head"),
        ("muscle pain", "Muscles"),
        ("fever", "General"),
        ("sneezing", "Throat & nose"),
        ("itchy eyes", "Eyes"),
        ("heartburn", "Stomach"),
        ("nausea", "Stomach"),
        ("fatigue", "Sleep & energy"),
        ("insomnia", "Sleep & energy"),
    ];
    for (symptom, category) in LINKS {
        let Some((_, id)) = ids.iter().find(|(name, _)| *name == category) else {
            continue;
        };
        tx.execute(
            "UPDATE symptoms SET category_id = ?2 WHERE name = ?1",
            params![symptom, id],
        )
        .map_err(crate::store::db_err)?;
    }
    Ok(())
}

fn med(name: &str, purpose: &str) -> MedicineInput {
    MedicineInput {
        name: name.into(),
        purpose: purpose.into(),
        form_id: None,
        strength: String::new(),
        notes: String::new(),
        active: true,
        as_needed: false,
        min_gap_minutes: None,
        max_per_day: None,
        stock: 0.0,
        stock_unit: String::new(),
        low_stock_at: None,
        expires_on: None,
        opened_on: None,
        shelf_life_days: None,
        course_ends_on: None,
        symptoms: Vec::new(),
    }
}

fn slot(time: &str) -> SlotInput {
    SlotInput {
        time: time.into(),
        days: 0b111_1111,
        dose: 1.0,
    }
}

fn words(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_string()).collect()
}

fn demo_medicines(today: Date) -> Vec<(MedicineInput, Vec<SlotInput>)> {
    let later = |days: i64| Some(today.plus_days(days).iso());
    let mut ibuprofen = med("Ibuprofen", "Pain and fever");
    ibuprofen.strength = "400 mg".into();
    ibuprofen.as_needed = true;
    (ibuprofen.min_gap_minutes, ibuprofen.max_per_day) = (Some(360), Some(3));
    (
        ibuprofen.stock,
        ibuprofen.stock_unit,
        ibuprofen.low_stock_at,
    ) = (12.0, "tablets".into(), Some(4.0));
    ibuprofen.expires_on = later(20);
    ibuprofen.symptoms = words(&["headache", "toothache", "fever", "muscle pain"]);

    let mut paracetamol = med("Paracetamol", "Pain and fever");
    paracetamol.strength = "500 mg".into();
    paracetamol.as_needed = true;
    (paracetamol.min_gap_minutes, paracetamol.max_per_day) = (Some(240), Some(4));
    (
        paracetamol.stock,
        paracetamol.stock_unit,
        paracetamol.low_stock_at,
    ) = (2.0, "tablets".into(), Some(4.0));
    paracetamol.expires_on = later(400);
    paracetamol.symptoms = words(&["headache", "fever"]);

    let mut vitamin = med("Vitamin D", "Daily supplement");
    (vitamin.stock, vitamin.stock_unit, vitamin.low_stock_at) =
        (60.0, "capsules".into(), Some(10.0));
    vitamin.expires_on = later(300);
    vitamin.symptoms = words(&["fatigue"]);

    let mut amoxicillin = med("Amoxicillin", "Ear infection");
    amoxicillin.strength = "500 mg".into();
    (amoxicillin.stock, amoxicillin.stock_unit) = (14.0, "capsules".into());
    amoxicillin.course_ends_on = later(2);
    amoxicillin.symptoms = words(&["fever"]);

    let mut cetirizine = med("Cetirizine", "Hay fever");
    cetirizine.as_needed = true;
    (cetirizine.stock, cetirizine.stock_unit) = (0.0, "tablets".into());
    cetirizine.expires_on = later(200);
    cetirizine.symptoms = words(&["sneezing", "itchy eyes"]);

    let mut omeprazole = med("Omeprazole", "Stomach acid");
    omeprazole.active = false;
    (omeprazole.stock, omeprazole.stock_unit) = (28.0, "capsules".into());
    omeprazole.expires_on = later(-10);
    omeprazole.symptoms = words(&["heartburn", "nausea"]);

    let mut melatonin = med("Melatonin", "Sleep");
    (
        melatonin.stock,
        melatonin.stock_unit,
        melatonin.low_stock_at,
    ) = (30.0, "tablets".into(), Some(7.0));
    melatonin.expires_on = later(500);
    melatonin.symptoms = words(&["insomnia"]);

    vec![
        (ibuprofen, vec![]),
        (paracetamol, vec![]),
        (vitamin, vec![slot("08:00")]),
        (amoxicillin, vec![slot("08:00"), slot("20:00")]),
        (cetirizine, vec![]),
        (omeprazole, vec![slot("07:30")]),
        (melatonin, vec![slot("22:30")]),
    ]
}
