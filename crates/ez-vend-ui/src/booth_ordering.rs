#![allow(clippy::let_and_return)]

use std::cmp::Ordering;

use domain::models::booth::Booth;

/// Apply consistent ordering for booth lists:
/// 1. Date descending (newest first)
/// 2. Tie-breaker: description ascending (case-insensitive)
pub fn sort_booths(booths: &mut [Booth]) {
    booths.sort_by(compare_booths);
}

pub fn split_booths(booths: &[Booth]) -> (Vec<Booth>, Vec<Booth>) {
    let mut active = booths
        .iter()
        .filter(|booth| !booth.is_archived())
        .cloned()
        .collect::<Vec<_>>();
    let mut archived = booths
        .iter()
        .filter(|booth| booth.is_archived())
        .cloned()
        .collect::<Vec<_>>();

    sort_booths(&mut active);
    archived.sort_by(|left, right| match (left.archived_at, right.archived_at) {
        (Some(left), Some(right)) => right.cmp(&left),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => compare_booths(left, right),
    });

    (active, archived)
}

fn compare_booths(a: &Booth, b: &Booth) -> Ordering {
    match b.date.cmp(&a.date) {
        Ordering::Equal => {}
        other => return other,
    }

    let a_desc = a.description.to_lowercase();
    let b_desc = b.description.to_lowercase();
    a_desc.cmp(&b_desc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use domain::models::booth::{Booth, FeeConfig};
    use rust_decimal::Decimal;

    fn test_booth(desc: &str, date: &str) -> Booth {
        let fees = FeeConfig {
            participation_fee: Decimal::ONE,
            sales_fee_percent: Decimal::ONE,
            rounding_step: Decimal::ONE,
        };

        let booth = Booth::new(
            desc.to_string(),
            NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap(),
            fees,
        )
        .unwrap();
        booth
    }

    #[test]
    fn sorts_by_date_and_description() {
        let mut booths = vec![
            test_booth("Alpha", "2026-03-25"),
            test_booth("Beta", "2026-03-26"),
            test_booth("Gamma", "2026-03-24"),
            test_booth("beta", "2026-03-26"),
            test_booth("Omega", "2026-03-28"),
        ];

        sort_booths(&mut booths);

        let ordered: Vec<_> = booths.into_iter().map(|b| b.description).collect();
        assert_eq!(ordered, vec!["Omega", "Beta", "beta", "Alpha", "Gamma"]);
    }

    #[test]
    fn split_booths_separates_archived_from_active() {
        let mut archived = test_booth("Archived", "2026-03-20");
        archived.archived_at = Some(chrono::Utc::now());

        let booths = vec![test_booth("Active", "2026-03-21"), archived.clone()];
        let (active, archived_list) = split_booths(&booths);

        assert_eq!(active.len(), 1);
        assert_eq!(archived_list.len(), 1);
        assert_eq!(active[0].description, "Active");
        assert_eq!(archived_list[0].description, archived.description);
    }
}
