//! P&L returns computed from the daily snapshot series.

use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;

/// Returns over the day, month to date, year to date and since inception.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pnl {
    pub day: Decimal,
    pub mtd: Decimal,
    pub ytd: Decimal,
    pub inception: Decimal,
}

fn ret(base: Decimal, now: Decimal) -> Decimal {
    now.checked_div(base)
        .map_or(Decimal::ZERO, |r| r - Decimal::ONE)
}

/// Compute the returns as of `asof` from `(date, equity)` snapshots sorted by date.
///
/// Month and year to date are measured from the last snapshot of the previous
/// month or year, falling back to the first snapshot. Returns `None` when there
/// is no snapshot for `asof`.
pub fn pnl(snapshots: &[(NaiveDate, Decimal)], asof: NaiveDate) -> Option<Pnl> {
    let upto: Vec<_> = snapshots.iter().filter(|(d, _)| *d <= asof).collect();
    let (last_date, now) = *upto.last()?;
    if *last_date != asof {
        return None;
    }
    let first = upto[0].1;
    let last_before = |keep: &dyn Fn(NaiveDate) -> bool| {
        upto.iter()
            .rev()
            .find(|(d, _)| keep(*d))
            .map_or(first, |(_, v)| *v)
    };
    let prev = if upto.len() >= 2 {
        upto[upto.len() - 2].1
    } else {
        *now
    };
    let month_start = (asof.year(), asof.month());
    let mtd_base = last_before(&|d| (d.year(), d.month()) < month_start);
    let ytd_base = last_before(&|d| d.year() < asof.year());
    Some(Pnl {
        day: ret(prev, *now),
        mtd: ret(mtd_base, *now),
        ytd: ret(ytd_base, *now),
        inception: ret(first, *now),
    })
}
