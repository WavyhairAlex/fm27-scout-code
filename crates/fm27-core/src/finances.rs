//! Club finance snapshots and corporate facilities, read from each club record.
//!
//! A club that keeps a finance series stores a counted chain of fixed 49-byte rows
//! inside its own record. The last row is the month before the save's clock month,
//! except on a month's last day, where the save already holds that month's row.
//! Only the clubs of the tracked league nations keep a series; most clubs yield no
//! row at all.

use std::collections::HashMap;

use chrono::{Datelike, Months, NaiveDate};

use crate::clubs::{ClubIndex, ClubRecordSpan};
use crate::layout::{FinanceLayout, read_i32, read_u8, read_u32};
use crate::model::FinanceMonth;

#[derive(Debug, Clone, Default)]
pub struct ClubFinanceData {
    pub months: Vec<FinanceMonth>,
    /// `Some(value)` when the club's record has a finance chain and a rating in the
    /// 1..=20 range; `None` when the club has no chain or stores an unset rating.
    pub corporate_facilities: HashMap<u32, Option<u8>>,
}

#[derive(Debug, Clone, Copy)]
pub struct FinanceChainLocation {
    pub head: usize,
    pub row_count: usize,
}

impl FinanceChainLocation {
    pub fn end(&self) -> usize {
        self.head + FinanceLayout::ROW_BYTES * self.row_count
    }

    pub fn facility_offset(&self) -> usize {
        self.end() + FinanceLayout::FACILITY_OFFSET_AFTER_CHAIN
    }

    pub fn last_row(&self) -> usize {
        self.head + FinanceLayout::ROW_BYTES * (self.row_count - 1)
    }
}

/// Read every club's monthly finance rows and corporate facilities rating.
pub fn read_club_finances(
    game_db: &[u8],
    club_index: &ClubIndex,
    clock: Option<NaiveDate>,
) -> ClubFinanceData {
    let mut data = ClubFinanceData::default();
    for (&club_uid, &span) in &club_index.record_spans {
        let Some(chain) = locate_finance_chain(game_db, span) else {
            continue;
        };
        let club_name = club_index
            .club(club_uid)
            .map(|club| club.name.clone())
            .unwrap_or_default();
        let labels = month_labels(clock, chain.row_count);
        for (index, month) in labels.into_iter().enumerate() {
            let row = chain.head + FinanceLayout::ROW_BYTES * index;
            let Some(row) = read_finance_row(game_db, row) else {
                continue;
            };
            data.months.push(FinanceMonth {
                club_uid,
                club_name: club_name.clone(),
                month,
                balance: row.balance,
                transfer_budget_allocated: row.transfer_budget_allocated,
                transfer_budget_remaining: row.transfer_budget_remaining,
                wage_budget_weekly: row.wage_budget_weekly,
                wage_payroll_weekly: row.wage_payroll_weekly,
                income_excluding_transfers: row.income_excluding_transfers,
                net_transfers: row.net_transfers,
                wage_bill: row.wage_bill,
                net: row.net,
                expenditure_excluding_transfers: row.expenditure_excluding_transfers,
                total_income: row.total_income,
                total_expenditure: row.total_expenditure,
            });
        }
        let facility_offset = chain.facility_offset();
        if facility_offset < span.record_end {
            if let Some(value) = read_u8(game_db, facility_offset) {
                let (lowest, highest) = FinanceLayout::FACILITY_RANGE;
                data.corporate_facilities.insert(
                    club_uid,
                    (lowest..=highest).contains(&value).then_some(value),
                );
            }
        }
    }
    data.months.sort_by(|left, right| {
        left.club_uid
            .cmp(&right.club_uid)
            .then_with(|| left.month.cmp(&right.month))
    });
    data
}

/// Locate one club's counted finance chain.
pub fn locate_finance_chain(game_db: &[u8], span: ClubRecordSpan) -> Option<FinanceChainLocation> {
    if span.record_end <= span.record_start
        || span.record_end - span.record_start < FinanceLayout::MINIMUM_RECORD_BYTES
    {
        return None;
    }
    let search_start = span.record_start.saturating_add(4);
    let search_end = span.record_end.min(game_db.len());
    if search_start >= search_end {
        return None;
    }
    let mut search_from = search_start;
    let mut first_short: Option<FinanceChainLocation> = None;
    while let Some(relative) = find_byte(&game_db[search_from..search_end], FinanceLayout::TAG) {
        let head = search_from + relative;
        search_from = head + 1;
        let Some(count_at) = head.checked_sub(4) else {
            continue;
        };
        let Some(row_count) = read_u32(game_db, count_at).map(|value| value as usize) else {
            continue;
        };
        let Some(chain_end) = head.checked_add(FinanceLayout::ROW_BYTES.checked_mul(row_count)?)
        else {
            continue;
        };
        if chain_end > search_end {
            continue;
        }
        let (full_low, full_high) = FinanceLayout::COUNT_RANGE;
        if (full_low as usize..=full_high as usize).contains(&row_count) {
            if rows_check_out(game_db, head, row_count, false) {
                return Some(FinanceChainLocation { head, row_count });
            }
            continue;
        }
        let (short_low, short_high) = FinanceLayout::SHORT_COUNT_RANGE;
        if first_short.is_none()
            && (short_low as usize..=short_high as usize).contains(&row_count)
            && rows_check_out(game_db, head, row_count, true)
        {
            first_short = Some(FinanceChainLocation { head, row_count });
        }
    }
    first_short
}

fn rows_check_out(game_db: &[u8], head: usize, row_count: usize, balanced: bool) -> bool {
    let mut money_moved = !balanced;
    for index in 0..row_count {
        let row = head + FinanceLayout::ROW_BYTES * index;
        if read_u8(game_db, row) != Some(FinanceLayout::TAG) {
            return false;
        }
        let Some(balance) = read_i32(game_db, row + FinanceLayout::BALANCE_OFFSET) else {
            return false;
        };
        let (lowest, highest) = FinanceLayout::BALANCE_RANGE;
        if !(lowest..=highest).contains(&balance) {
            return false;
        }
        let Some(wage_budget) = read_u32(game_db, row + FinanceLayout::WAGE_BUDGET_OFFSET) else {
            return false;
        };
        let Some(wage_payroll) = read_u32(game_db, row + FinanceLayout::WAGE_PAYROLL_OFFSET) else {
            return false;
        };
        if wage_budget > FinanceLayout::WEEKLY_MAXIMUM
            || wage_payroll > FinanceLayout::WEEKLY_MAXIMUM
        {
            return false;
        }
        if balanced {
            let net = read_i32(game_db, row + FinanceLayout::NET_OFFSET);
            let total_income = read_i32(game_db, row + FinanceLayout::TOTAL_INCOME_OFFSET);
            let total_expenditure =
                read_i32(game_db, row + FinanceLayout::TOTAL_EXPENDITURE_OFFSET);
            let income_excluding = read_i32(
                game_db,
                row + FinanceLayout::INCOME_EXCLUDING_TRANSFERS_OFFSET,
            );
            let expenditure_excluding = read_i32(
                game_db,
                row + FinanceLayout::EXPENDITURE_EXCLUDING_TRANSFERS_OFFSET,
            );
            let (Some(net), Some(total_income), Some(total_expenditure)) =
                (net, total_income, total_expenditure)
            else {
                return false;
            };
            if net != total_income - total_expenditure {
                return false;
            }
            if income_excluding.is_some_and(|value| value < 0 || value > total_income) {
                return false;
            }
            if expenditure_excluding.is_some_and(|value| value < 0 || value > total_expenditure) {
                return false;
            }
            money_moved = money_moved || total_income != 0 || total_expenditure != 0;
        }
    }
    money_moved
}

struct FinanceRow {
    balance: i64,
    transfer_budget_allocated: i64,
    transfer_budget_remaining: i64,
    wage_budget_weekly: i64,
    wage_payroll_weekly: i64,
    income_excluding_transfers: i64,
    net_transfers: i64,
    wage_bill: i64,
    net: i64,
    expenditure_excluding_transfers: i64,
    total_income: i64,
    total_expenditure: i64,
}

fn read_finance_row(game_db: &[u8], row: usize) -> Option<FinanceRow> {
    Some(FinanceRow {
        balance: read_i32(game_db, row + FinanceLayout::BALANCE_OFFSET)? as i64,
        transfer_budget_allocated: read_i32(
            game_db,
            row + FinanceLayout::TRANSFER_ALLOCATED_OFFSET,
        )? as i64,
        transfer_budget_remaining: read_i32(
            game_db,
            row + FinanceLayout::TRANSFER_REMAINING_OFFSET,
        )? as i64,
        wage_budget_weekly: read_u32(game_db, row + FinanceLayout::WAGE_BUDGET_OFFSET)? as i64,
        wage_payroll_weekly: read_u32(game_db, row + FinanceLayout::WAGE_PAYROLL_OFFSET)? as i64,
        income_excluding_transfers: read_i32(
            game_db,
            row + FinanceLayout::INCOME_EXCLUDING_TRANSFERS_OFFSET,
        )? as i64,
        net_transfers: read_i32(game_db, row + FinanceLayout::NET_TRANSFERS_OFFSET)? as i64,
        wage_bill: read_i32(game_db, row + FinanceLayout::WAGE_BILL_OFFSET)? as i64,
        net: read_i32(game_db, row + FinanceLayout::NET_OFFSET)? as i64,
        expenditure_excluding_transfers: read_i32(
            game_db,
            row + FinanceLayout::EXPENDITURE_EXCLUDING_TRANSFERS_OFFSET,
        )? as i64,
        total_income: read_i32(game_db, row + FinanceLayout::TOTAL_INCOME_OFFSET)? as i64,
        total_expenditure: read_i32(game_db, row + FinanceLayout::TOTAL_EXPENDITURE_OFFSET)? as i64,
    })
}

fn month_labels(clock: Option<NaiveDate>, count: usize) -> Vec<String> {
    let Some(clock) = clock else {
        return (0..count).map(|index| format!("unknown-{index}")).collect();
    };
    let save_month = NaiveDate::from_ymd_opt(clock.year(), clock.month(), 1).unwrap_or(clock);
    let lag = if is_last_day_of_month(clock) { 0 } else { 1 };
    let last = save_month
        .checked_sub_months(Months::new(lag))
        .unwrap_or(save_month);
    (0..count)
        .map(|index| {
            let months_back = (count - 1 - index) as u32;
            last.checked_sub_months(Months::new(months_back))
                .unwrap_or(last)
                .format("%Y-%m-%d")
                .to_string()
        })
        .collect()
}

fn is_last_day_of_month(date: NaiveDate) -> bool {
    date.succ_opt()
        .is_none_or(|next| next.month() != date.month() || next.year() != date.year())
}

fn find_byte(haystack: &[u8], needle: u8) -> Option<usize> {
    haystack.iter().position(|value| *value == needle)
}

/// Decode the stored facility byte at a known offset. The editor uses this to verify
/// that the offset still lands on a plausible rating.
pub fn read_corporate_facilities(game_db: &[u8], chain: FinanceChainLocation) -> Option<u8> {
    let value = read_u8(game_db, chain.facility_offset())?;
    let (lowest, highest) = FinanceLayout::FACILITY_RANGE;
    (lowest..=highest).contains(&value).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn month_labels_end_at_the_month_before_the_clock() {
        let clock = NaiveDate::from_ymd_opt(2026, 10, 24).unwrap();
        assert_eq!(
            month_labels(Some(clock), 4),
            vec!["2026-06-01", "2026-07-01", "2026-08-01", "2026-09-01"]
        );
    }

    #[test]
    fn a_last_day_save_already_holds_that_month() {
        let clock = NaiveDate::from_ymd_opt(2026, 10, 31).unwrap();
        assert_eq!(
            month_labels(Some(clock), 4),
            vec!["2026-07-01", "2026-08-01", "2026-09-01", "2026-10-01"]
        );
    }

    #[test]
    fn locates_a_fixed_finance_chain_and_facilities_byte() {
        let mut game_db = vec![0u8; FinanceLayout::MINIMUM_RECORD_BYTES + 100];
        let head = 8usize;
        let count = 3u32;
        game_db[head - 4..head].copy_from_slice(&count.to_le_bytes());
        for index in 0..count as usize {
            let row = head + FinanceLayout::ROW_BYTES * index;
            game_db[row] = FinanceLayout::TAG;
            game_db[row + FinanceLayout::BALANCE_OFFSET..row + FinanceLayout::BALANCE_OFFSET + 4]
                .copy_from_slice(&(1_000_000i32 + index as i32).to_le_bytes());
            game_db[row + FinanceLayout::WAGE_BUDGET_OFFSET
                ..row + FinanceLayout::WAGE_BUDGET_OFFSET + 4]
                .copy_from_slice(&500_000u32.to_le_bytes());
            game_db[row + FinanceLayout::WAGE_PAYROLL_OFFSET
                ..row + FinanceLayout::WAGE_PAYROLL_OFFSET + 4]
                .copy_from_slice(&450_000u32.to_le_bytes());
        }
        let chain = locate_finance_chain(
            &game_db,
            ClubRecordSpan {
                record_start: 0,
                record_end: game_db.len(),
            },
        )
        .expect("chain");
        assert_eq!(chain.head, head);
        assert_eq!(chain.row_count, 3);
        let facility = chain.facility_offset();
        game_db[facility] = 12;
        assert_eq!(read_corporate_facilities(&game_db, chain), Some(12));
    }
}
