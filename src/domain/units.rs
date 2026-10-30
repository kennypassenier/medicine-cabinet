//! Human wording for an amount and its unit.

/// "1 capsule", "2 tablets", "5 ml": the unit is singular when exactly one.
pub fn amount(dose: f64, unit: &str) -> String {
    let number = if dose.fract() == 0.0 {
        format!("{dose:.0}")
    } else {
        format!("{dose}")
    };
    let unit = unit.trim();
    if unit.is_empty() {
        return number;
    }
    let shown = if (dose - 1.0).abs() < f64::EPSILON {
        singular(unit)
    } else {
        unit.to_string()
    };
    format!("{number} {shown}")
}

/// Singular of the plural units the forms use ("tablets", "patches", "puffs").
fn singular(unit: &str) -> String {
    if unit.ends_with("ches") || unit.ends_with("shes") {
        return unit[..unit.len() - 2].to_string();
    }
    match unit.strip_suffix('s') {
        Some(stem) if stem.len() > 2 => stem.to_string(),
        _ => unit.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_is_singular_many_is_plural() {
        assert_eq!(amount(1.0, "capsules"), "1 capsule");
        assert_eq!(amount(2.0, "capsules"), "2 capsules");
        assert_eq!(amount(1.0, "patches"), "1 patch");
    }

    #[test]
    fn short_and_missing_units_stay_as_they_are() {
        assert_eq!(amount(5.0, "ml"), "5 ml");
        assert_eq!(amount(1.0, "ml"), "1 ml");
        assert_eq!(amount(0.5, ""), "0.5");
    }
}
