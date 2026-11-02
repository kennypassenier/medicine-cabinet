use super::*;

fn input() -> MedicineInput {
    serde_json::from_str(r#"{"name":"Ibuprofen"}"#).unwrap()
}

#[test]
fn a_plain_medicine_passes_and_is_trimmed() {
    let mut m = input();
    m.name = "  Ibuprofen \u{e9} ".into();
    assert_eq!(m.validated().unwrap().name, "Ibuprofen \u{e9}");
}

#[test]
fn names_and_texts_are_bounded() {
    let mut m = input();
    m.name = "x".repeat(121);
    assert!(m.validated().is_err());
    let mut m = input();
    m.name = "   ".into();
    assert!(m.validated().is_err());
    let mut m = input();
    m.name = "line\nbreak".into();
    assert!(m.validated().is_err());
    let mut m = input();
    m.notes = "x".repeat(2001);
    assert!(m.validated().is_err());
    let mut m = input();
    m.symptoms = vec!["s".into(); 101];
    assert!(m.validated().is_err());
}

#[test]
fn numbers_must_be_sane() {
    for stock in [-1.0, f64::NAN, f64::INFINITY, 1e12] {
        let mut m = input();
        m.stock = stock;
        assert!(m.validated().is_err(), "stock {stock}");
    }
    let cases = [
        (Some(0), None, None),
        (Some(99_999), None, None),
        (None, Some(0), None),
        (None, Some(5000), None),
        (None, None, Some(0)),
        (None, None, Some(4_000_000_000)),
    ];
    for (gap, per_day, shelf) in cases {
        let mut m = input();
        (m.min_gap_minutes, m.max_per_day, m.shelf_life_days) = (gap, per_day, shelf);
        assert!(m.validated().is_err(), "{gap:?} {per_day:?} {shelf:?}");
    }
}

#[test]
fn dates_are_checked_and_normalised() {
    let mut m = input();
    m.expires_on = Some(" 2027-01-31T10:00 ".into());
    assert_eq!(
        m.validated().unwrap().expires_on.as_deref(),
        Some("2027-01-31")
    );
    for bad in ["2027-02-30", "9999-01-01", "tomorrow", "0000-00-00"] {
        let mut m = input();
        m.course_ends_on = Some(bad.into());
        assert!(m.validated().is_err(), "{bad}");
    }
}
