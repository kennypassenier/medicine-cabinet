//! Today's attention list: stock, expiry and course alerts.

use super::*;
use crate::testkit::medicine;

fn today() -> Date {
    Date::parse("2026-10-30").unwrap()
}

fn kinds(all: &[Medicine]) -> Vec<(i64, &'static str)> {
    attention_for(all, today())
        .into_iter()
        .map(|a| (a.medicine_id, a.kind))
        .collect()
}

#[test]
fn a_healthy_medicine_needs_no_attention() {
    assert!(kinds(&[medicine(1, "A")]).is_empty());
}

#[test]
fn stock_alerts_distinguish_out_from_low() {
    let mut out = medicine(1, "A");
    out.stock_level = StockLevel::Out;
    let mut low = medicine(2, "B");
    low.stock_level = StockLevel::Low;
    low.stock = 2.5;
    let list = attention_for(&[out, low], today());
    let got: Vec<_> = list.iter().map(|a| (a.medicine_id, a.kind)).collect();
    assert_eq!(got, [(1, "out_of_stock"), (2, "low_stock")]);
    assert_eq!(list[1].detail, "Only 2.5 capsules left.");
}

#[test]
fn expiry_alerts_need_a_date_to_name() {
    let mut expired = medicine(1, "A");
    expired.freshness = Freshness::Expired;
    expired.effective_expiry = Some("2026-10-01".into());
    let mut soon = medicine(2, "B");
    soon.freshness = Freshness::ExpiringSoon;
    soon.effective_expiry = Some("2026-11-10".into());
    let mut dateless = medicine(3, "C");
    dateless.freshness = Freshness::Expired;
    let list = attention_for(&[expired, soon, dateless], today());
    assert_eq!(
        list.iter()
            .map(|a| (a.medicine_id, a.kind))
            .collect::<Vec<_>>(),
        [(1, "expired"), (2, "expiring_soon")]
    );
    assert_eq!(list[0].detail, "Expired on 2026-10-01.");
    assert_eq!(list[1].detail, "Good until 2026-11-10.");
}

#[test]
fn a_course_alerts_from_three_days_out_through_its_last_day() {
    for (offset, alert) in [(-1, false), (0, true), (3, true), (4, false)] {
        let mut m = medicine(1, "A");
        m.course_ends_on = Some(today().plus_days(offset).iso());
        assert_eq!(kinds(&[m]).len(), usize::from(alert), "offset {offset}");
    }
}

#[test]
fn a_course_end_text_names_the_date() {
    let mut m = medicine(1, "A");
    m.course_ends_on = Some("2026-11-01".into());
    assert_eq!(
        attention_for(&[m], today())[0].detail,
        "The course ends on 2026-11-01."
    );
}

#[test]
fn switched_off_medicines_raise_nothing() {
    let mut m = medicine(1, "A");
    m.active = false;
    m.stock_level = StockLevel::Out;
    m.freshness = Freshness::Expired;
    m.effective_expiry = Some("2026-01-01".into());
    assert!(kinds(&[m]).is_empty());
}

#[test]
fn one_medicine_can_raise_several_alerts_in_a_fixed_order() {
    let mut m = medicine(1, "A");
    m.stock_level = StockLevel::Low;
    m.freshness = Freshness::ExpiringSoon;
    m.effective_expiry = Some("2026-11-05".into());
    m.course_ends_on = Some("2026-11-01".into());
    assert_eq!(
        kinds(&[m]),
        [
            (1, "low_stock"),
            (1, "expiring_soon"),
            (1, "course_ends_soon")
        ]
    );
}

#[test]
fn an_unparseable_course_date_is_ignored() {
    let mut m = medicine(1, "A");
    m.course_ends_on = Some("not-a-date".into());
    assert!(kinds(&[m]).is_empty());
}
