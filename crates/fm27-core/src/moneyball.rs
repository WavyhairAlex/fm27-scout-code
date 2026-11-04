//! Transparent value-for-money ranking from data the save already provides.
//!
//! This is the first Moneyball iteration: a "value finder", not a stats-based
//! model. It compares role quality and total cost inside position groups, so a
//! cheap player with a good role fit can be found without pretending that CA or
//! transfer value alone is an answer. Season statistics can later be layered on
//! top without changing the request/response shape.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

pub const VALUE_FINDER_MODEL: &str = "value-finder-v1";
const COST_HORIZON_YEARS: f64 = 3.0;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ValueFinderRequest {
    pub save_id: i64,
    pub position_group: Option<String>,
    pub league: Option<String>,
    pub age_min: Option<i32>,
    pub age_max: Option<i32>,
    pub ca_min: Option<u16>,
    pub ca_max: Option<u16>,
    pub role_min: Option<u8>,
    pub max_fee: Option<u64>,
    pub max_wage: Option<u64>,
    pub contract_end_before: Option<String>,
    /// Any of `sale`, `loan` and `free`.
    pub listings: Vec<String>,
    pub min_confidence: Option<f64>,
    pub exclude_injured: bool,
    pub exclude_injury_history: bool,
    pub include_unknown_fee: bool,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueFinderRow {
    pub uid: u32,
    pub name: Option<String>,
    pub age: Option<i32>,
    pub club_uid: Option<u32>,
    pub club_name: Option<String>,
    pub league_name: Option<String>,
    pub nation_name: Option<String>,
    pub ca: u16,
    pub pa: Option<i16>,
    pub role_id: Option<String>,
    pub role_name: Option<String>,
    pub role_score: Option<u8>,
    pub role_position: Option<String>,
    pub position_group: String,
    pub transfer_value: Option<u32>,
    pub transfer_value_state: String,
    pub wage: Option<u32>,
    pub contract_end: Option<String>,
    pub club_reputation: Option<u16>,
    pub fee: Option<u64>,
    pub cost: u64,
    pub fee_known: bool,
    pub wage_known: bool,
    pub role_pct: Option<f64>,
    pub ca_pct: Option<f64>,
    pub quality_pct: f64,
    pub cost_pct: Option<f64>,
    pub surplus: Option<f64>,
    pub score: f64,
    pub confidence: f64,
    pub league_median_ca: Option<f64>,
    pub flags: Vec<String>,
    pub transfer_listed: bool,
    pub loan_listed: bool,
    pub injury_status: String,
    pub recent_injury: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueFinderMeta {
    pub model: String,
    pub scored: usize,
    pub filtered: usize,
    pub cost_horizon_years: f64,
    pub leagues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueFinderPage {
    pub total: usize,
    pub page: u32,
    pub page_size: u32,
    pub rows: Vec<ValueFinderRow>,
    pub meta: ValueFinderMeta,
}

/// One player row loaded from the store before peer percentiles are computed.
#[derive(Debug, Clone)]
pub struct ValueFinderCandidate {
    pub uid: u32,
    pub name: Option<String>,
    pub age: Option<i32>,
    pub club_uid: Option<u32>,
    pub club_name: Option<String>,
    pub league_name: Option<String>,
    pub nation_name: Option<String>,
    pub ca: u16,
    pub pa: Option<i16>,
    pub role_id: Option<String>,
    pub role_name: Option<String>,
    pub role_score: Option<u8>,
    pub role_position: Option<String>,
    pub transfer_value: Option<u32>,
    pub transfer_value_state: String,
    pub wage: Option<u32>,
    pub contract_end: Option<String>,
    pub club_reputation: Option<u16>,
    pub transfer_listed: bool,
    pub loan_listed: bool,
    pub injury_status: String,
    pub recent_injury: Option<String>,
}

/// Group a role position into the six groups the UI compares inside.
pub fn position_group(position: Option<&str>) -> &'static str {
    let Some(position) = position else {
        return "OTHER";
    };
    match position.trim().to_ascii_uppercase().as_str() {
        "GK" => "GK",
        "DC" | "SW" => "DC",
        "DL" | "DR" | "WBL" | "WBR" => "FB",
        "DM" | "MC" | "ML" | "MR" => "MID",
        "AMC" | "AML" | "AMR" => "AM",
        "STC" | "ST" | "SC" => "ST",
        _ => "OTHER",
    }
}

/// Average-rank percentile: ties share their average rank and the result is in
/// `0..=1`. `None` stays `None` so callers can distinguish "unknown" from
/// "bottom of the group".
pub(crate) fn percentiles(values: &[Option<f64>]) -> Vec<Option<f64>> {
    let mut present: Vec<(usize, f64)> = values
        .iter()
        .enumerate()
        .filter_map(|(index, value)| value.map(|value| (index, value)))
        .collect();
    let mut out = vec![None; values.len()];
    if present.is_empty() {
        return out;
    }
    present.sort_by(|left, right| left.1.partial_cmp(&right.1).unwrap_or(Ordering::Equal));
    let denominator = (present.len() - 1).max(1) as f64;
    let mut start = 0;
    while start < present.len() {
        let mut end = start + 1;
        while end < present.len() && (present[end].1 - present[start].1).abs() <= f64::EPSILON {
            end += 1;
        }
        let rank = ((start + end - 1) as f64 / 2.0) / denominator;
        for (index, _) in &present[start..end] {
            out[*index] = Some(rank);
        }
        start = end;
    }
    out
}

fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|left, right| left.partial_cmp(right).unwrap_or(Ordering::Equal));
    let middle = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        Some((sorted[middle - 1] + sorted[middle]) / 2.0)
    } else {
        Some(sorted[middle])
    }
}

fn fee_for(candidate: &ValueFinderCandidate) -> (Option<u64>, bool) {
    let state = candidate.transfer_value_state.as_str();
    if state == "ok"
        && let Some(value) = candidate.transfer_value
    {
        return (Some(value as u64), true);
    }
    if candidate.club_uid.is_none() && state == "zero" {
        return (Some(0), true);
    }
    (None, false)
}

fn cost_for(fee: Option<u64>, wage: Option<u32>) -> Option<u64> {
    // A free agent with no known wage is not a zero-cost player: the wage
    // demand is simply unknown. Treating that as zero would put every free
    // agent at the top of the list.
    if fee.is_none() && wage.is_none() {
        return None;
    }
    if wage.is_none() && fee.is_some_and(|value| value == 0) {
        return None;
    }
    let fee = fee.unwrap_or(0);
    let wages = wage.unwrap_or(0) as u64 * 52 * COST_HORIZON_YEARS as u64;
    Some(fee.saturating_add(wages))
}

fn flags_for(
    candidate: &ValueFinderCandidate,
    fee_known: bool,
    wage_known: bool,
    confidence: f64,
) -> Vec<String> {
    let mut flags = Vec::new();
    if !fee_known {
        flags.push("missing_fee".to_owned());
    }
    if !wage_known {
        flags.push("missing_wage".to_owned());
    }
    if candidate.club_uid.is_none() {
        flags.push("free_agent".to_owned());
    }
    if candidate.transfer_listed {
        flags.push("transfer_listed".to_owned());
    }
    if candidate.loan_listed {
        flags.push("loan_listed".to_owned());
    }
    if candidate.injury_status == "injured" {
        flags.push("injured".to_owned());
    }
    if candidate.recent_injury.is_some() {
        flags.push("injury_history".to_owned());
    }
    if candidate.league_name.is_none() {
        flags.push("unknown_league".to_owned());
    }
    if candidate.role_score.is_none() {
        flags.push("unknown_role".to_owned());
    }
    if confidence < 0.7 {
        flags.push("low_confidence".to_owned());
    }
    flags
}

fn confidence_for(candidate: &ValueFinderCandidate, fee_known: bool, wage_known: bool) -> f64 {
    let mut confidence: f64 = 0.55;
    if fee_known {
        confidence += 0.15;
    }
    if wage_known {
        confidence += 0.15;
    }
    if candidate.role_score.is_some() {
        confidence += 0.10;
    }
    if candidate.club_reputation.is_some() {
        confidence += 0.05;
    }
    confidence.clamp(0.0, 1.0)
}

fn sort_value(row: &ValueFinderRow, by: &str) -> Option<f64> {
    match by {
        "score" => Some(row.score),
        "surplus" => row.surplus,
        "quality" => Some(row.quality_pct),
        "cost" => Some(row.cost as f64),
        "fee" | "value" => row.fee.map(|value| value as f64),
        "wage" => row.wage.map(|value| value as f64),
        "age" => row.age.map(|value| value as f64),
        "ca" => Some(row.ca as f64),
        "role" => row.role_score.map(|value| value as f64),
        "confidence" => Some(row.confidence),
        _ => Some(row.score),
    }
}

fn compare_rows(
    left: &ValueFinderRow,
    right: &ValueFinderRow,
    by: &str,
    ascending: bool,
) -> Ordering {
    if by == "name" {
        let order = left
            .name
            .as_deref()
            .unwrap_or("")
            .cmp(right.name.as_deref().unwrap_or(""));
        let order = if ascending { order } else { order.reverse() };
        return order.then_with(|| left.uid.cmp(&right.uid));
    }
    let left_value = sort_value(left, by);
    let right_value = sort_value(right, by);
    // Unknown values sort last regardless of direction.
    match (left_value, right_value) {
        (None, None) => return left.uid.cmp(&right.uid),
        (None, Some(_)) => return Ordering::Greater,
        (Some(_), None) => return Ordering::Less,
        (Some(left_number), Some(right_number)) => {
            let order = left_number
                .partial_cmp(&right_number)
                .unwrap_or(Ordering::Equal);
            let order = if ascending { order } else { order.reverse() };
            return order.then_with(|| left.uid.cmp(&right.uid));
        }
    }
}

/// Request-independent Value Finder scoring. Percentiles are computed from the
/// complete candidate set, so request filters can be applied without moving ranks.
#[derive(Debug, Clone)]
pub struct ValueFinderBaseline {
    rows: Vec<ValueFinderRow>,
    leagues: Vec<String>,
}

pub fn validate_value_request(request: &ValueFinderRequest) -> Result<()> {
    if let Some(group) = request
        .position_group
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_uppercase())
        && !matches!(group.as_str(), "GK" | "DC" | "FB" | "MID" | "AM" | "ST")
    {
        return Err(Error::parse(format!("unknown position group {group:?}")));
    }
    for listing in &request.listings {
        if !matches!(listing.as_str(), "sale" | "loan" | "free") {
            return Err(Error::parse(format!("unknown listing filter {listing:?}")));
        }
    }
    Ok(())
}

pub fn score_value_baseline(candidates: Vec<ValueFinderCandidate>) -> ValueFinderBaseline {
    let groups: Vec<&'static str> = candidates
        .iter()
        .map(|candidate| position_group(candidate.role_position.as_deref()))
        .collect();
    let leagues: Vec<String> = {
        let mut names = std::collections::BTreeSet::new();
        for candidate in &candidates {
            if let Some(league) = candidate.league_name.as_deref() {
                names.insert(league.to_owned());
            }
        }
        names.into_iter().collect()
    };

    // League context is a display aid, not a hidden multiplier. A player in a
    // weak league should not win solely because his league is weak.
    let mut league_cas: std::collections::HashMap<String, Vec<f64>> =
        std::collections::HashMap::new();
    for candidate in &candidates {
        if let Some(league) = candidate.league_name.as_deref() {
            league_cas
                .entry(league.to_owned())
                .or_default()
                .push(candidate.ca as f64);
        }
    }
    let league_medians: std::collections::HashMap<String, f64> = league_cas
        .into_iter()
        .filter_map(|(league, values)| median(&values).map(|value| (league, value)))
        .collect();

    let role_values: Vec<Option<f64>> = candidates
        .iter()
        .map(|candidate| candidate.role_score.map(|value| value as f64))
        .collect();
    let ca_values: Vec<Option<f64>> = candidates
        .iter()
        .map(|candidate| Some(candidate.ca as f64))
        .collect();

    // Percentiles are computed inside each position group. This keeps a
    // full-back from being compared with a striker, and makes role scores
    // meaningful despite their different absolute ranges.
    let mut role_pct = vec![None; candidates.len()];
    let mut ca_pct = vec![None; candidates.len()];
    let mut cost_pct = vec![None; candidates.len()];
    for group in ["GK", "DC", "FB", "MID", "AM", "ST", "OTHER"] {
        let indices: Vec<usize> = groups
            .iter()
            .enumerate()
            .filter_map(|(index, value)| (*value == group).then_some(index))
            .collect();
        if indices.is_empty() {
            continue;
        }
        let group_roles: Vec<Option<f64>> =
            indices.iter().map(|index| role_values[*index]).collect();
        let group_cas: Vec<Option<f64>> = indices.iter().map(|index| ca_values[*index]).collect();
        let group_costs: Vec<Option<f64>> = indices
            .iter()
            .map(|index| {
                let candidate = &candidates[*index];
                let (fee, _) = fee_for(candidate);
                cost_for(fee, candidate.wage).map(|cost| (cost as f64).ln_1p())
            })
            .collect();
        let role_ranks = percentiles(&group_roles);
        let ca_ranks = percentiles(&group_cas);
        let cost_ranks = percentiles(&group_costs);
        for (offset, index) in indices.iter().enumerate() {
            role_pct[*index] = role_ranks[offset];
            ca_pct[*index] = ca_ranks[offset];
            cost_pct[*index] = cost_ranks[offset];
        }
    }

    let mut rows = Vec::with_capacity(candidates.len());
    for (index, candidate) in candidates.into_iter().enumerate() {
        let group = position_group(candidate.role_position.as_deref());
        let (fee, fee_known) = fee_for(&candidate);
        let wage_known = candidate.wage.is_some();
        let Some(cost) = cost_for(fee, candidate.wage) else {
            continue;
        };
        if cost == 0 {
            continue;
        }
        let role_pct = role_pct[index];
        let ca_pct = ca_pct[index];
        let quality_pct = match (role_pct, ca_pct) {
            (Some(role), Some(ca)) => 0.6 * role + 0.4 * ca,
            (Some(role), None) => role,
            (None, Some(ca)) => ca,
            (None, None) => 0.0,
        };
        let cost_pct = cost_pct[index];
        let surplus = cost_pct.map(|cost| quality_pct - cost);
        let score = 100.0
            * match cost_pct {
                Some(cost) => 0.55 * quality_pct + 0.45 * (1.0 - cost),
                None => quality_pct,
            };
        let confidence = confidence_for(&candidate, fee_known, wage_known);
        let flags = flags_for(&candidate, fee_known, wage_known, confidence);
        let league_median_ca = candidate
            .league_name
            .as_deref()
            .and_then(|league| league_medians.get(league).copied());

        rows.push(ValueFinderRow {
            uid: candidate.uid,
            name: candidate.name,
            age: candidate.age,
            club_uid: candidate.club_uid,
            club_name: candidate.club_name,
            league_name: candidate.league_name,
            nation_name: candidate.nation_name,
            ca: candidate.ca,
            pa: candidate.pa,
            role_id: candidate.role_id,
            role_name: candidate.role_name,
            role_score: candidate.role_score,
            role_position: candidate.role_position,
            position_group: group.to_owned(),
            transfer_value: candidate.transfer_value,
            transfer_value_state: candidate.transfer_value_state,
            wage: candidate.wage,
            contract_end: candidate.contract_end,
            club_reputation: candidate.club_reputation,
            fee,
            cost,
            fee_known,
            wage_known,
            role_pct,
            ca_pct,
            quality_pct,
            cost_pct,
            surplus,
            score,
            confidence,
            league_median_ca,
            flags,
            transfer_listed: candidate.transfer_listed,
            loan_listed: candidate.loan_listed,
            injury_status: candidate.injury_status,
            recent_injury: candidate.recent_injury,
        });
    }

    ValueFinderBaseline { rows, leagues }
}

fn matches_value_request(
    row: &ValueFinderRow,
    request: &ValueFinderRequest,
    position_filter: Option<&str>,
) -> bool {
    if !request.include_unknown_fee && !row.fee_known {
        return false;
    }
    if let Some(group) = position_filter
        && row.position_group != group
    {
        return false;
    }
    if let Some(league) = request.league.as_deref()
        && row.league_name.as_deref() != Some(league)
    {
        return false;
    }
    if let Some(min) = request.age_min
        && row.age.is_none_or(|age| age < min)
    {
        return false;
    }
    if let Some(max) = request.age_max
        && row.age.is_none_or(|age| age > max)
    {
        return false;
    }
    if let Some(min) = request.ca_min
        && row.ca < min
    {
        return false;
    }
    if let Some(max) = request.ca_max
        && row.ca > max
    {
        return false;
    }
    if let Some(min) = request.role_min
        && row.role_score.is_none_or(|score| score < min)
    {
        return false;
    }
    if let Some(max_fee) = request.max_fee {
        match row.fee {
            Some(value) if value <= max_fee => {}
            Some(_) => return false,
            None if request.include_unknown_fee => {}
            None => return false,
        }
    }
    if let Some(max_wage) = request.max_wage
        && row.wage.is_none_or(|value| value as u64 > max_wage)
    {
        return false;
    }
    if let Some(before) = request.contract_end_before.as_deref()
        && row.contract_end.as_deref().is_none_or(|end| end > before)
    {
        return false;
    }
    if !request.listings.is_empty()
        && !request
            .listings
            .iter()
            .any(|listing| matches_row_listing(row, listing))
    {
        return false;
    }
    if let Some(min) = request.min_confidence
        && row.confidence < min
    {
        return false;
    }
    if request.exclude_injured && row.injury_status == "injured" {
        return false;
    }
    if request.exclude_injury_history && row.recent_injury.is_some() {
        return false;
    }
    true
}

fn matches_row_listing(row: &ValueFinderRow, listing: &str) -> bool {
    match listing {
        "sale" => row.transfer_listed,
        "loan" => row.loan_listed,
        "free" => row.club_uid.is_none(),
        _ => false,
    }
}

pub fn apply_value_request(
    baseline: &ValueFinderBaseline,
    request: &ValueFinderRequest,
) -> Result<ValueFinderPage> {
    validate_value_request(request)?;
    let position_filter = request
        .position_group
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_uppercase());

    let mut rows: Vec<&ValueFinderRow> = baseline
        .rows
        .iter()
        .filter(|row| matches_value_request(row, request, position_filter.as_deref()))
        .collect();
    let filtered = rows.len();
    let by = request.sort_by.as_deref().unwrap_or("score");
    let ascending = request
        .sort_dir
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case("asc"));
    rows.sort_by(|left, right| compare_rows(left, right, by, ascending));

    let page = request.page.unwrap_or(0);
    let page_size = request.page_size.unwrap_or(100).clamp(1, 500);
    let start = (page as usize).saturating_mul(page_size as usize);
    let rows = if start >= rows.len() {
        Vec::new()
    } else {
        rows[start..(start + page_size as usize).min(rows.len())]
            .iter()
            .map(|row| (*row).clone())
            .collect()
    };
    Ok(ValueFinderPage {
        total: filtered,
        page,
        page_size,
        rows,
        meta: ValueFinderMeta {
            model: VALUE_FINDER_MODEL.to_owned(),
            scored: filtered,
            filtered,
            cost_horizon_years: COST_HORIZON_YEARS,
            leagues: baseline.leagues.clone(),
        },
    })
}

/// Score candidates and return one page. Kept as a convenience for callers that
/// do not need to reuse the baseline across requests.
pub fn score_value_finder(
    candidates: Vec<ValueFinderCandidate>,
    request: &ValueFinderRequest,
) -> Result<ValueFinderPage> {
    validate_value_request(request)?;
    let baseline = score_value_baseline(candidates);
    apply_value_request(&baseline, request)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(uid: u32, role: u8, ca: u16, fee: u32, wage: u32) -> ValueFinderCandidate {
        ValueFinderCandidate {
            uid,
            name: Some(format!("Player {uid}")),
            age: Some(24),
            club_uid: Some(10),
            club_name: Some("Club".to_owned()),
            league_name: Some("League".to_owned()),
            nation_name: Some("Nation".to_owned()),
            ca,
            pa: Some(ca as i16 + 10),
            role_id: Some("central_midfielder".to_owned()),
            role_name: Some("Central Midfielder".to_owned()),
            role_score: Some(role),
            role_position: Some("MC".to_owned()),
            transfer_value: Some(fee),
            transfer_value_state: "ok".to_owned(),
            wage: Some(wage),
            contract_end: Some("2028-06-30".to_owned()),
            club_reputation: Some(5000),
            transfer_listed: false,
            loan_listed: false,
            injury_status: "fit".to_owned(),
            recent_injury: None,
        }
    }

    fn request() -> ValueFinderRequest {
        ValueFinderRequest {
            save_id: 1,
            ..ValueFinderRequest::default()
        }
    }

    #[test]
    fn position_groups_cover_the_role_positions() {
        assert_eq!(position_group(Some("GK")), "GK");
        assert_eq!(position_group(Some("DC")), "DC");
        assert_eq!(position_group(Some("WBR")), "FB");
        assert_eq!(position_group(Some("DM")), "MID");
        assert_eq!(position_group(Some("AML")), "AM");
        assert_eq!(position_group(Some("STC")), "ST");
        assert_eq!(position_group(None), "OTHER");
    }

    #[test]
    fn percentile_ranks_keep_order_and_share_ties() {
        let ranks = percentiles(&[Some(10.0), Some(10.0), Some(20.0), None]);
        assert_eq!(ranks[0], Some(0.25));
        assert_eq!(ranks[1], Some(0.25));
        assert_eq!(ranks[2], Some(1.0));
        assert_eq!(ranks[3], None);
    }

    #[test]
    fn cheap_role_fit_beats_expensive_equal_quality() {
        let candidates = vec![
            candidate(1, 16, 140, 1_000_000, 20_000),
            candidate(2, 16, 140, 50_000_000, 200_000),
            candidate(3, 12, 110, 500_000, 10_000),
        ];
        let page = score_value_finder(candidates, &request()).expect("score");
        assert_eq!(page.total, 3);
        assert_eq!(page.rows[0].uid, 1);
        assert!(page.rows[0].surplus.unwrap() > page.rows[1].surplus.unwrap());
    }

    #[test]
    fn unknown_fee_is_flagged_and_not_treated_as_free() {
        let mut player = candidate(1, 14, 130, 0, 10_000);
        player.transfer_value_state = "placeholder".to_owned();
        let mut query = request();
        query.include_unknown_fee = true;
        let page = score_value_finder(vec![player], &query).expect("score");
        assert!(!page.rows[0].fee_known);
        assert_eq!(page.rows[0].fee, None);
        assert!(page.rows[0].flags.iter().any(|flag| flag == "missing_fee"));
        assert!(page.rows[0].confidence < 1.0);
    }

    #[test]
    fn zero_cost_rows_are_not_ranked() {
        let page =
            score_value_finder(vec![candidate(1, 14, 130, 0, 0)], &request()).expect("score");
        assert_eq!(page.total, 0);
    }

    #[test]
    fn injury_filters_exclude_unavailable_players() {
        let mut injured = candidate(1, 16, 150, 1_000_000, 20_000);
        injured.injury_status = "injured".to_owned();
        injured.recent_injury = Some("2026-08-01".to_owned());
        let mut history = candidate(2, 16, 150, 1_000_000, 20_000);
        history.recent_injury = Some("2026-08-01".to_owned());
        let fit = candidate(3, 16, 150, 1_000_000, 20_000);

        let mut query = request();
        query.exclude_injured = true;
        let page = score_value_finder(vec![injured.clone(), history.clone(), fit.clone()], &query)
            .expect("score");
        assert_eq!(page.total, 2);
        assert!(page.rows.iter().all(|row| row.uid != 1));

        query.exclude_injury_history = true;
        let page = score_value_finder(vec![injured, history, fit], &query).expect("score");
        assert_eq!(page.total, 1);
        assert_eq!(page.rows[0].uid, 3);
    }

    #[test]
    fn free_agent_without_known_wage_is_not_zero_cost() {
        let mut player = candidate(1, 14, 130, 0, 10_000);
        player.club_uid = None;
        player.wage = None;
        player.transfer_value_state = "zero".to_owned();
        let page = score_value_finder(vec![player], &request()).expect("score");
        assert_eq!(page.total, 0);
    }

    #[test]
    fn free_agent_with_known_wage_stays_ranked() {
        let mut player = candidate(1, 14, 130, 0, 10_000);
        player.club_uid = None;
        player.transfer_value_state = "zero".to_owned();
        let page = score_value_finder(vec![player], &request()).expect("score");
        assert_eq!(page.total, 1);
        assert_eq!(page.rows[0].fee, Some(0));
        assert!(page.rows[0].flags.iter().any(|flag| flag == "free_agent"));
    }

    #[test]
    fn unknown_fee_is_excluded_by_default() {
        let mut player = candidate(1, 14, 130, 0, 10_000);
        player.transfer_value_state = "unset".to_owned();
        let page = score_value_finder(vec![player], &request()).expect("score");
        assert_eq!(page.total, 0);
    }

    #[test]
    fn fee_filter_keeps_known_fees_and_can_include_unknown() {
        let mut unknown = candidate(2, 14, 130, 0, 10_000);
        unknown.transfer_value_state = "unset".to_owned();
        let candidates = vec![candidate(1, 14, 130, 1_000, 10_000), unknown];
        let mut query = request();
        query.max_fee = Some(2_000);
        let page = score_value_finder(candidates.clone(), &query).expect("score");
        assert_eq!(page.total, 1);
        assert_eq!(page.rows[0].uid, 1);

        query.include_unknown_fee = true;
        let page = score_value_finder(candidates, &query).expect("score");
        assert_eq!(page.total, 2);
    }

    #[test]
    fn missing_league_and_role_are_explicit_flags() {
        let mut player = candidate(1, 14, 130, 1_000, 10_000);
        player.league_name = None;
        player.role_score = None;
        player.role_position = None;
        let page = score_value_finder(vec![player], &request()).expect("score");
        let flags = &page.rows[0].flags;
        assert!(flags.iter().any(|flag| flag == "unknown_league"));
        assert!(flags.iter().any(|flag| flag == "unknown_role"));
        assert_eq!(page.rows[0].position_group, "OTHER");
    }

    #[test]
    fn pagination_and_sorting_are_stable() {
        let candidates = (0..5)
            .map(|index| {
                candidate(
                    index + 1,
                    10 + index as u8,
                    100 + index as u16,
                    1_000,
                    1_000,
                )
            })
            .collect();
        let mut query = request();
        query.page_size = Some(2);
        query.page = Some(1);
        query.sort_by = Some("ca".to_owned());
        query.sort_dir = Some("desc".to_owned());
        let page = score_value_finder(candidates, &query).expect("score");
        assert_eq!(page.total, 5);
        assert_eq!(page.rows.len(), 2);
        assert_eq!(page.rows[0].uid, 3);
        assert_eq!(page.rows[1].uid, 2);
    }

    #[test]
    fn unknown_position_group_is_rejected() {
        let mut query = request();
        query.position_group = Some("WING".to_owned());
        let error = score_value_finder(Vec::new(), &query).expect_err("reject");
        assert!(error.to_string().contains("unknown position group"));
    }
}
