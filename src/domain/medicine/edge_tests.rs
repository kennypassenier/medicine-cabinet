//! Medicine validation and the freshness / stock judgements at their edges.

use super::*;

fn d(s: &str) -> Date {
    Date::parse(s).unwrap()
}

fn input(name: &str) -> MedicineInput {
    serde_json::from_value(serde_json::json!({ "name": name })).unwrap()
}

#[test]
fn a_minimal_json_body_gets_the_documented_defaults() {
    let m = input("Aspirin");
    assert!(m.active, "new medicines start switched on");
    assert!(!m.as_needed);
    assert_eq!(m.stock, 0.0);
    assert!(m.symptoms.is_empty());
    assert!(m.expires_on.is_none());
}

#[test]
fn the_name_is_trimmed_and_must_not_be_blank() {
    assert_eq!(input("  Aspirin \t").validated().unwrap().name, "Aspirin");
    for blank in ["", "   ", "\t\n"] {
        let e = input(blank).validated().unwrap_err();
        assert!(e.message.contains("name"), "{}", e.message);
    }
}

#[test]
fn negative_stock_or_threshold_is_refused_but_zero_is_fine() {
    let mut m = input("A");
    m.stock = -0.5;
    assert!(m.validated().is_err());
    let mut m = input("A");
    m.low_stock_at = Some(-1.0);
    assert!(m.validated().is_err());
    let mut m = input("A");
    m.stock = 0.0;
    m.low_stock_at = Some(0.0);
    assert!(m.validated().is_ok());
}

#[test]
fn dates_are_normalised_blank_ones_become_none() {
    let mut m = input("A");
    m.expires_on = Some(" 2027-01-31 ".into());
    m.opened_on = Some("".into());
    m.course_ends_on = Some("   ".into());
    let v = m.validated().unwrap();
    assert_eq!(v.expires_on.as_deref(), Some("2027-01-31"));
    assert_eq!(v.opened_on, None);
    assert_eq!(v.course_ends_on, None);
}

#[test]
fn a_date_with_a_time_part_is_stored_as_a_plain_date() {
    let mut m = input("A");
    m.opened_on = Some("2026-10-30T08:00".into());
    assert_eq!(
        m.validated().unwrap().opened_on.as_deref(),
        Some("2026-10-30")
    );
}

#[test]
fn any_one_bad_date_refuses_the_whole_input() {
    for which in 0..3 {
        let mut m = input("A");
        let bad = Some("2026-02-30".to_string());
        match which {
            0 => m.expires_on = bad,
            1 => m.opened_on = bad,
            _ => m.course_ends_on = bad,
        }
        assert!(m.validated().is_err(), "field {which}");
    }
}

#[test]
fn symptoms_are_cleaned_during_validation() {
    let mut m = input("A");
    m.symptoms = vec!["  Cough".into(), "COUGH".into(), " ".into()];
    assert_eq!(m.validated().unwrap().symptoms, vec!["cough"]);
}

#[test]
fn clean_symptoms_keeps_first_seen_order_and_handles_empty_input() {
    assert!(clean_symptoms(&[]).is_empty());
    let raw: Vec<String> = ["b", "a", "B", "c", "A"].map(String::from).to_vec();
    assert_eq!(clean_symptoms(&raw), vec!["b", "a", "c"]);
}

#[test]
fn freshness_boundaries_are_today_thirty_and_thirty_one_days() {
    let today = d("2026-10-30");
    assert_eq!(freshness(Some(today), today), Freshness::ExpiringSoon);
    assert_eq!(
        freshness(Some(today.plus_days(-1)), today),
        Freshness::Expired
    );
    assert_eq!(
        freshness(Some(today.plus_days(30)), today),
        Freshness::ExpiringSoon
    );
    assert_eq!(
        freshness(Some(today.plus_days(31)), today),
        Freshness::Fresh
    );
}

#[test]
fn freshness_counts_across_a_year_boundary() {
    let today = d("2026-12-15");
    assert_eq!(
        freshness(Some(d("2027-01-14")), today),
        Freshness::ExpiringSoon
    );
    assert_eq!(freshness(Some(d("2027-01-15")), today), Freshness::Fresh);
}

#[test]
fn effective_expiry_uses_whichever_source_exists() {
    assert_eq!(effective_expiry(None, None, None), None);
    assert_eq!(
        effective_expiry(Some("2027-01-01"), None, None),
        Some(d("2027-01-01"))
    );
    assert_eq!(
        effective_expiry(None, Some("2026-10-01"), Some(10)),
        Some(d("2026-10-11"))
    );
    // Opened without a shelf life (or the reverse) says nothing.
    assert_eq!(effective_expiry(None, Some("2026-10-01"), None), None);
    assert_eq!(effective_expiry(None, None, Some(10)), None);
}

#[test]
fn when_the_printed_date_is_earlier_it_wins() {
    let e = effective_expiry(Some("2026-10-05"), Some("2026-10-01"), Some(90));
    assert_eq!(e, Some(d("2026-10-05")));
}

#[test]
fn shelf_life_crosses_a_leap_day() {
    let e = effective_expiry(None, Some("2024-02-28"), Some(2));
    assert_eq!(e, Some(d("2024-03-01")));
}

#[test]
fn stock_level_edges() {
    assert_eq!(
        stock_level(5.0, Some(5.0)),
        StockLevel::Low,
        "at the threshold"
    );
    assert_eq!(stock_level(5.1, Some(5.0)), StockLevel::Ok);
    assert_eq!(stock_level(0.0, None), StockLevel::Out);
    assert_eq!(stock_level(-1.0, None), StockLevel::Out);
    assert_eq!(stock_level(0.5, Some(0.0)), StockLevel::Ok);
    assert_eq!(
        stock_level(0.0, Some(10.0)),
        StockLevel::Out,
        "out beats low"
    );
}
