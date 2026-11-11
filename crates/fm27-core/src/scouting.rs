//! Transfer advice for the club the save's human manager runs.
//!
//! Two questions stay separate: whether a player can play a position (fit) and
//! whether he would join the managed club (probability). Fit uses the existing
//! role weights, then applies position familiarity. Probability is an explicit,
//! additive estimate built from the transfer value, contract, club reputation,
//! wage room and budget, so every recommendation can explain itself.

use chrono::{Days, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::layout::POSITION_KEYS;
use crate::model::{Attributes, Clause, Personality, Positions, TransferValueState};
use crate::roles::{ROLES, RoleSpec, raw_role_score};

/// The transfer value the game gives a player who is not for sale.
pub const NOT_FOR_SALE: u32 = 300_000_000;
/// The lowest position rating a recommendation may have.
pub const LOWEST_POSITION_RATING: u8 = 10;
/// The youngest a future prospect may be.
pub const PROSPECT_AGE: i32 = 21;
/// How many fitting players are measured in detail before the list is cut.
const SHORTLIST: usize = 400;
const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 200;
/// Release clause kinds (Minimum Fee and its variants), as FM27 numbers them.
const RELEASE_CLAUSES: [u16; 10] = [0, 1, 2, 16, 17, 18, 27, 31, 55, 56];
/// A release clause condition past twenty years is a mark, not a day count.
const MOST_RELEASE_CLAUSE_DAYS: u16 = 7_305;
/// The age past which a free agent is most likely retired.
const FREE_AGENT_AGE_LIMIT: i32 = 35;

const SCOUT_POSITION_KEYS: [&str; 14] = [
    "gk", "dr", "dc", "dl", "wbr", "wbl", "dm", "mc", "mr", "ml", "amr", "amc", "aml", "stc",
];

struct LineSpec {
    key: &'static str,
    positions: &'static [&'static str],
    target: usize,
}

/// The lines of a senior squad and how many players it usually carries there.
const LINES: &[LineSpec] = &[
    LineSpec {
        key: "gk",
        positions: &["gk"],
        target: 2,
    },
    LineSpec {
        key: "dc",
        positions: &["dc"],
        target: 4,
    },
    LineSpec {
        key: "rightBack",
        positions: &["dr", "wbr"],
        target: 2,
    },
    LineSpec {
        key: "leftBack",
        positions: &["dl", "wbl"],
        target: 2,
    },
    LineSpec {
        key: "dm",
        positions: &["dm"],
        target: 2,
    },
    LineSpec {
        key: "mc",
        positions: &["mc"],
        target: 3,
    },
    LineSpec {
        key: "rightWing",
        positions: &["amr", "mr"],
        target: 2,
    },
    LineSpec {
        key: "leftWing",
        positions: &["aml", "ml"],
        target: 2,
    },
    LineSpec {
        key: "amc",
        positions: &["amc"],
        target: 2,
    },
    LineSpec {
        key: "st",
        positions: &["stc"],
        target: 3,
    },
];

/// What a recommendation is for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScoutPurpose {
    /// Better than the club's best player there now.
    #[default]
    FirstTeam,
    /// At least as good as its second best, at a sensible price.
    Rotation,
    /// 21 or younger, with room to grow.
    Future,
    /// Anyone who fits.
    Any,
}

/// A player the scout recommends.
#[derive(Debug, Clone, Serialize)]
pub struct ScoutPlayer {
    pub uid: u32,
    pub name: Option<String>,
    pub age: Option<i32>,
    pub club_uid: Option<u32>,
    pub club_name: Option<String>,
    pub ca: u16,
    pub pa: Option<i16>,
    pub transfer_value: Option<u32>,
    pub wage: Option<u32>,
    pub contract_end: Option<String>,
    pub transfer_listed: bool,
}

/// How a player measures up to a position.
#[derive(Debug, Clone, Serialize)]
pub struct ScoutFitting {
    pub player: ScoutPlayer,
    /// 0-100: his attributes for the role, held back by position familiarity.
    pub fit: u8,
    pub role_id: String,
    pub role_name: String,
    /// The raw role rating, 1-20, before position familiarity.
    pub role_score: f32,
    pub position_rating: u8,
}

/// Why a player looks more or less likely to join, with the value it is about.
#[derive(Debug, Clone, Serialize)]
pub struct ScoutReason {
    /// `listed`, `freeAgent`, `expiring`, `releaseClause`, `fringe`, `keyPlayer`,
    /// `unwanted`, `overBudget`, `stretch`, `wageHigh`, `biggerClub`,
    /// `smallerClub`, `reputationGap`, `ambitious`, `notForSale`.
    pub code: &'static str,
    /// Its effect on the probability, in points.
    pub points: i32,
    /// The amount or count it names: pounds, days, a reputation.
    pub value: Option<i64>,
}

/// A scouted player with his likelihood and expected cost.
#[derive(Debug, Clone, Serialize)]
pub struct ScoutSuggestion {
    #[serde(flatten)]
    pub fitting: ScoutFitting,
    /// 0-100. An estimate, not the game's own.
    pub probability: u8,
    pub reasons: Vec<ScoutReason>,
    /// Likely fee and weekly wage in pounds; none when unknown.
    pub fee: Option<i64>,
    pub wage: Option<i64>,
    /// The game's transfer value; none when unset or not for sale.
    pub value: Option<u32>,
    /// Fit and probability together, as the purpose weighs them.
    pub score: u8,
    /// The club that would sell him when it is not the one he plays for.
    pub owner_club: Option<String>,
    pub owner_club_uid: Option<u32>,
}

/// The recommendations and what they were measured against.
#[derive(Debug, Clone, Serialize)]
pub struct Scouted {
    pub suggestions: Vec<ScoutSuggestion>,
    /// The club's own players for the position, best first (at most five).
    pub squad: Vec<ScoutFitting>,
    pub transfer_budget: Option<i64>,
    pub wage_budget: Option<i64>,
    pub wage_bill: Option<i64>,
    pub wage_room: Option<i64>,
    /// Players looked at, those who can play the position, and those who match
    /// the purpose.
    pub examined: usize,
    pub candidates: usize,
    pub eligible: usize,
    pub game_date: Option<String>,
}

/// What the scout needs to know about the buying club.
#[derive(Debug, Clone)]
pub struct ScoutContext {
    pub club_uid: u32,
    pub club_name: String,
    pub club_gender: Option<u8>,
    pub club_reputation: Option<u16>,
    pub transfer_budget: Option<i64>,
    pub wage_budget: Option<i64>,
    pub wage_bill: Option<i64>,
    pub game_date: Option<NaiveDate>,
}

impl ScoutContext {
    pub fn wage_room(&self) -> Option<i64> {
        self.wage_budget
            .zip(self.wage_bill)
            .map(|(budget, bill)| budget - bill)
    }
}

/// What a recommendation is asked for.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ScoutRequest {
    pub save_id: i64,
    pub club_uid: u32,
    /// A position key among [`SCOUT_POSITION_KEYS`].
    pub position: String,
    /// A role id; none means the best role for each player at the position.
    pub role_id: Option<String>,
    pub purpose: ScoutPurpose,
    pub min_age: Option<i32>,
    pub max_age: Option<i32>,
    /// The most the club would pay, pounds.
    pub max_fee: Option<i64>,
    /// Only players at least this likely (0-100) to join.
    pub min_probability: Option<u8>,
    pub no_free_agents: bool,
    pub how_many: Option<usize>,
}

/// One player as the scout's input. Built by the store from the save cache.
#[derive(Debug, Clone)]
pub struct ScoutCandidate {
    pub uid: u32,
    pub name: Option<String>,
    pub age: Option<i32>,
    pub club_uid: Option<u32>,
    pub club_name: Option<String>,
    pub contract_club_uid: Option<u32>,
    pub contract_club_name: Option<String>,
    pub gender: Option<u8>,
    pub ca: u16,
    pub pa: Option<i16>,
    pub attributes: Attributes,
    pub positions: Positions,
    pub personality: Option<Personality>,
    pub transfer_value: Option<u32>,
    pub transfer_value_state: TransferValueState,
    pub transfer_listed: bool,
    pub wage: Option<u32>,
    pub contract_start: Option<NaiveDate>,
    pub contract_end: Option<NaiveDate>,
    pub squad_status: Option<u8>,
    pub clauses: Vec<Clause>,
    pub on_loan: bool,
    pub loan_parent_club_uid: Option<u32>,
    pub loan_parent_club_name: Option<String>,
    pub reputation: Option<u16>,
    pub club_reputation: Option<u16>,
}

/// A role offered by the editor, with the positions it covers.
#[derive(Debug, Clone, Serialize)]
pub struct ScoutRole {
    pub id: &'static str,
    pub name: &'static str,
    pub positions: Vec<&'static str>,
}

/// How a squad line's numbers look.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DepthStatus {
    Missing,
    Thin,
    Fine,
    Crowded,
}

/// A player of one squad line.
#[derive(Debug, Clone, Serialize)]
pub struct ScoutDepthPlayer {
    pub player: ScoutPlayer,
    /// His best rating (1-20) among the line's positions.
    pub rating: u8,
    pub on_loan: bool,
}

/// One line of the senior squad.
#[derive(Debug, Clone, Serialize)]
pub struct ScoutLine {
    pub key: &'static str,
    pub positions: Vec<&'static str>,
    pub target: usize,
    /// The players whose best position is in this line, best first.
    pub players: Vec<ScoutDepthPlayer>,
    /// Players rated 15 or more here whose best position is elsewhere.
    pub cover: Vec<ScoutDepthPlayer>,
    pub status: DepthStatus,
    /// Its best player is well below the squad's level.
    pub weak: bool,
    /// All its players are 31 or older.
    pub ageing: bool,
}

/// What the scout would do with a player he would let go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DepartureAction {
    Sell,
    Loan,
    Release,
}

/// Why a player could go, with the value it is about.
#[derive(Debug, Clone, Serialize)]
pub struct DepartureReason {
    /// `surplus`, `belowLevel`, `ageing`, `expiring`, `highWage`, `blocked`,
    /// `unwanted`, `listed`.
    pub code: &'static str,
    pub value: Option<i64>,
}

/// A player the scout would let go.
#[derive(Debug, Clone, Serialize)]
pub struct ScoutDeparture {
    pub player: ScoutPlayer,
    pub action: DepartureAction,
    pub reasons: Vec<DepartureReason>,
    /// The more reasons and the stronger, the higher.
    pub priority: u16,
    pub value: Option<u32>,
    pub not_for_sale: bool,
    pub wage: Option<u32>,
    pub line: &'static str,
    /// Where he stands in his line, 1 is its best.
    pub rank: usize,
}

/// The Scouting view's fixed data and the managed club's senior squad.
#[derive(Debug, Clone, Serialize)]
pub struct ScoutOverview {
    pub club_uid: u32,
    pub club_name: String,
    pub transfer_budget: Option<i64>,
    pub wage_budget: Option<i64>,
    pub wage_bill: Option<i64>,
    pub wage_room: Option<i64>,
    pub squad_size: usize,
    pub average_age: Option<f64>,
    /// The average current ability of the squad's best eleven.
    pub level: u16,
    pub lines: Vec<ScoutLine>,
    pub departures: Vec<ScoutDeparture>,
    pub positions: Vec<&'static str>,
    pub roles: Vec<ScoutRole>,
}

/// Whether a request may name this position.
pub fn is_scout_position(key: &str) -> bool {
    SCOUT_POSITION_KEYS.contains(&key) && POSITION_KEYS.contains(&key)
}

/// The roles that can be played in at least one scout position.
pub fn scout_roles() -> Vec<ScoutRole> {
    ROLES
        .iter()
        .filter(|role| {
            role.positions
                .iter()
                .any(|position| is_scout_position(position))
        })
        .map(|role| ScoutRole {
            id: role.id,
            name: role.name,
            positions: role.positions.to_vec(),
        })
        .collect()
}

fn player_of(candidate: &ScoutCandidate) -> ScoutPlayer {
    ScoutPlayer {
        uid: candidate.uid,
        name: candidate.name.clone(),
        age: candidate.age,
        club_uid: candidate.club_uid,
        club_name: candidate.club_name.clone(),
        ca: candidate.ca,
        pa: candidate.pa,
        transfer_value: known_value(candidate),
        wage: candidate.wage,
        contract_end: candidate
            .contract_end
            .map(|date| date.format("%Y-%m-%d").to_string()),
        transfer_listed: candidate.transfer_listed,
    }
}

/// The value the game knows, or none when it is unset or the player is not for
/// sale. Unknown values never become zero.
pub fn known_value(candidate: &ScoutCandidate) -> Option<u32> {
    match candidate.transfer_value_state {
        TransferValueState::Ok => candidate
            .transfer_value
            .filter(|value| *value > 0 && *value != NOT_FOR_SALE),
        _ => None,
    }
}

fn not_for_sale(candidate: &ScoutCandidate) -> bool {
    candidate.transfer_value_state == TransferValueState::Ok
        && candidate.transfer_value == Some(NOT_FOR_SALE)
}

/// How much of a player's attributes he brings to a position rated `rating`.
fn position_factor(rating: u8) -> f32 {
    match rating {
        18.. => 1.0,
        15..=17 => 0.97,
        13..=14 => 0.88,
        10..=12 => 0.75,
        _ => 0.0,
    }
}

fn fit_of(raw: f32, rating: u8) -> u8 {
    ((raw - 5.0) / 12.0 * 100.0 * position_factor(rating))
        .round()
        .clamp(0.0, 100.0) as u8
}

fn role_for_position(
    position: &str,
    role_id: Option<&str>,
    attributes: &Attributes,
) -> Result<Option<&'static RoleSpec>> {
    if let Some(id) = role_id {
        let role = ROLES
            .iter()
            .find(|role| role.id == id)
            .ok_or_else(|| Error::parse(format!("unknown role {id:?}")))?;
        if !role.positions.contains(&position) {
            return Err(Error::parse(format!(
                "role {id:?} cannot play {position:?}"
            )));
        }
        return Ok(Some(role));
    }
    Ok(ROLES
        .iter()
        .filter(|role| role.positions.contains(&position))
        .max_by(|left, right| {
            raw_role_score(left, attributes)
                .total_cmp(&raw_role_score(right, attributes))
                .then_with(|| right.id.cmp(left.id))
        }))
}

fn fitting(
    candidate: &ScoutCandidate,
    position: &str,
    role_id: Option<&str>,
) -> Result<Option<ScoutFitting>> {
    let Some(rating) = candidate.positions.get(position) else {
        return Ok(None);
    };
    if rating < LOWEST_POSITION_RATING {
        return Ok(None);
    }
    let Some(role) = role_for_position(position, role_id, &candidate.attributes)? else {
        return Ok(None);
    };
    let raw = raw_role_score(role, &candidate.attributes);
    Ok(Some(ScoutFitting {
        player: player_of(candidate),
        fit: fit_of(raw, rating),
        role_id: role.id.to_owned(),
        role_name: role.name.to_owned(),
        role_score: (raw * 10.0).round() / 10.0,
        position_rating: rating,
    }))
}

fn potential(candidate: &ScoutCandidate) -> f32 {
    candidate
        .pa
        .map(|pa| (i64::from(pa) - 100).clamp(0, 100) as f32)
        .unwrap_or(0.0)
}

fn room_to_grow(candidate: &ScoutCandidate) -> i64 {
    candidate
        .pa
        .map(|pa| i64::from(pa) - i64::from(candidate.ca))
        .unwrap_or(0)
}

fn age_factor(age: i32) -> f32 {
    match age {
        ..=29 => 1.0,
        30..=31 => 0.95,
        32..=33 => 0.88,
        _ => 0.8,
    }
}

fn purpose_score(
    purpose: ScoutPurpose,
    fit: u8,
    probability: u8,
    candidate: &ScoutCandidate,
) -> u8 {
    let fit = f32::from(fit);
    let probability = f32::from(probability);
    let age = candidate.age.unwrap_or(27);
    let years = age_factor(age);
    let room = room_to_grow(candidate);
    let growth = if age <= 23 && room >= 15 {
        (room / 5).min(6) as f32
    } else {
        0.0
    };
    let score = match purpose {
        ScoutPurpose::FirstTeam | ScoutPurpose::Any => {
            (0.7 * fit + 0.3 * probability) * years + growth
        }
        ScoutPurpose::Rotation => (0.5 * fit + 0.5 * probability) * years,
        ScoutPurpose::Future => 0.35 * fit + 0.35 * potential(candidate) + 0.3 * probability,
    };
    score.round().clamp(0.0, 100.0) as u8
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Standing {
    Key,
    Regular,
    Fringe,
    Prospect,
    Surplus,
}

impl Standing {
    fn of(code: u8) -> Option<Self> {
        Some(match code {
            1 | 2 | 15 => Self::Key,
            3 => Self::Regular,
            4 | 5 | 7 | 9 | 14 | 16 | 17 | 20 | 21 => Self::Fringe,
            10 | 11 | 13 => Self::Prospect,
            22 => Self::Surplus,
            _ => return None,
        })
    }
}

fn days_between(from: NaiveDate, to: NaiveDate) -> i64 {
    to.signed_duration_since(from).num_days()
}

/// The lowest release clause that still holds, pounds.
fn release_fee(
    clauses: &[Clause],
    signed: Option<NaiveDate>,
    today: Option<NaiveDate>,
) -> Option<u32> {
    clauses
        .iter()
        .filter(|clause| RELEASE_CLAUSES.contains(&clause.kind))
        .filter(|clause| {
            let expired = match (clause.parameter, signed, today) {
                (Some(days), Some(signed), Some(today)) if days <= MOST_RELEASE_CLAUSE_DAYS => {
                    signed
                        .checked_add_days(Days::new(u64::from(days)))
                        .is_some_and(|expiry| today >= expiry)
                }
                _ => false,
            };
            !expired
        })
        .filter_map(|clause| clause.value)
        .filter(|amount| *amount > 0)
        .min()
}

/// A rough weekly wage for a player without a known contract, pounds.
fn going_wage(ca: u16) -> i64 {
    let ability = i64::from(ca.min(200));
    let base = (ability - 50).max(0);
    (base * base * base / 17).max(250)
}

fn probability(
    candidate: &ScoutCandidate,
    context: &ScoutContext,
    today: Option<NaiveDate>,
) -> (u8, Vec<ScoutReason>, Option<i64>, Option<i64>) {
    let mut reasons = Vec::new();
    let mut add = |code: &'static str, points: i32, value: Option<i64>| {
        reasons.push(ScoutReason {
            code,
            points,
            value,
        })
    };

    let free = candidate.club_uid.is_none();
    let value = known_value(candidate).map(i64::from);
    let not_for_sale = not_for_sale(candidate);
    let days_left = today
        .zip(candidate.contract_end)
        .map(|(today, end)| days_between(today, end));

    let mut fee = if free { Some(0) } else { value };
    if free {
        add("freeAgent", 20, None);
    } else if not_for_sale {
        add("notForSale", -40, None);
    }
    if !free {
        if candidate.transfer_listed {
            add("listed", 12, None);
            fee = fee.map(|fee| fee * 4 / 5);
        }
        match candidate.squad_status.and_then(Standing::of) {
            Some(Standing::Key) => {
                add("keyPlayer", -12, candidate.squad_status.map(i64::from));
                fee = fee.map(|fee| fee * 7 / 5);
            }
            Some(Standing::Fringe) => {
                add("fringe", 8, candidate.squad_status.map(i64::from));
                fee = fee.map(|fee| fee * 19 / 20);
            }
            Some(Standing::Surplus) => {
                add("unwanted", 12, candidate.squad_status.map(i64::from));
                fee = fee.map(|fee| fee * 17 / 20);
            }
            Some(Standing::Prospect) => {
                add("prospect", 4, candidate.squad_status.map(i64::from));
            }
            Some(Standing::Regular) | None => {
                fee = fee.map(|fee| fee * 11 / 10);
            }
        }
        match days_left {
            Some(days) if (0..=183).contains(&days) => {
                add("expiring", 12, Some(days));
                fee = fee.map(|fee| fee * 2 / 5);
            }
            Some(days) if (0..=365).contains(&days) => {
                add("expiring", 6, Some(days));
                fee = fee.map(|fee| fee * 7 / 10);
            }
            _ => {}
        }
        if let Some(clause) =
            release_fee(&candidate.clauses, candidate.contract_start, today).map(i64::from)
            && fee.is_none_or(|fee| clause < fee)
        {
            add("releaseClause", 5, Some(clause));
            fee = Some(clause);
        }
    }

    if let (Some(fee), Some(budget)) = (fee, context.transfer_budget) {
        if fee > budget * 13 / 10 {
            add("overBudget", -35, Some(fee - budget));
        } else if fee > budget {
            add("stretch", -12, Some(fee - budget));
        }
    }

    let wage = match candidate.wage {
        Some(wage) if wage > 0 => Some(i64::from(wage) * 5 / 4),
        _ => Some(going_wage(candidate.ca)),
    };
    if let (Some(wage), Some(room)) = (wage, context.wage_room())
        && wage > room.max(0)
    {
        add("wageHigh", -15, Some(wage - room.max(0)));
    }

    let ours = i64::from(context.club_reputation.unwrap_or(0));
    if let Some(theirs) = candidate.club_reputation.map(i64::from) {
        if theirs > ours + 1_000 {
            add("biggerClub", -18, Some(theirs));
        } else if theirs + 1_000 < ours {
            add("smallerClub", 8, Some(theirs));
        }
    }
    if let Some(reputation) = candidate.reputation.map(i64::from) {
        let gap = reputation - ours;
        if gap > 1_500 {
            let ambition = i64::from(candidate.personality.map_or(10, |value| value.ambition));
            let points = -((gap / 300).min(25) as i32) - ((ambition - 12).max(0) * 2) as i32;
            add("reputationGap", points, Some(reputation));
        } else if gap < -1_500
            && candidate
                .personality
                .is_some_and(|personality| personality.ambition >= 15)
        {
            add(
                "ambitious",
                4,
                candidate
                    .personality
                    .map(|personality| i64::from(personality.ambition)),
            );
        }
    }

    let points = 50 + reasons.iter().map(|reason| reason.points).sum::<i32>();
    (points.clamp(0, 100) as u8, reasons, fee, wage)
}

/// Recommend players for the position and purpose in `request`.
pub fn advise(
    request: &ScoutRequest,
    context: &ScoutContext,
    own: &[ScoutCandidate],
    candidates: &[ScoutCandidate],
) -> Result<Scouted> {
    let position = request.position.trim();
    if !is_scout_position(position) {
        return Err(Error::parse(format!("unknown position {position:?}")));
    }
    let role_id = request
        .role_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(id) = role_id {
        let role = ROLES
            .iter()
            .find(|role| role.id == id)
            .ok_or_else(|| Error::parse(format!("unknown role {id:?}")))?;
        if !role.positions.contains(&position) {
            return Err(Error::parse(format!(
                "role {id:?} cannot play {position:?}"
            )));
        }
    }

    let mut own_fits: Vec<ScoutFitting> = Vec::new();
    for candidate in own {
        if let Some(fitting) = fitting(candidate, position, role_id)?
            && fitting.fit > 0
        {
            own_fits.push(fitting);
        }
    }
    own_fits.sort_by(|left, right| {
        right
            .fit
            .cmp(&left.fit)
            .then_with(|| right.player.ca.cmp(&left.player.ca))
    });
    let best_own = own_fits.first().map_or(0, |fitting| fitting.fit);
    let second_own = own_fits.get(1).map_or(0, |fitting| fitting.fit);
    let squad: Vec<ScoutFitting> = own_fits.iter().take(5).cloned().collect();

    let mut examined = 0;
    let mut fitting_players: Vec<(ScoutCandidate, ScoutFitting, Option<u32>)> = Vec::new();
    for candidate in candidates {
        examined += 1;
        if candidate.club_uid == Some(context.club_uid)
            || candidate.contract_club_uid == Some(context.club_uid)
        {
            continue;
        }
        if candidate.loan_parent_club_uid == Some(context.club_uid) {
            continue;
        }
        let free = candidate.club_uid.is_none();
        if free && request.no_free_agents {
            continue;
        }
        if let (Some(gender), Some(ours)) = (candidate.gender, context.club_gender)
            && gender != ours
        {
            continue;
        }
        if request.purpose == ScoutPurpose::Future && candidate.age.is_none() {
            continue;
        }
        if let Some(age) = candidate.age {
            if request.min_age.is_some_and(|min| age < min)
                || request.max_age.is_some_and(|max| age > max)
            {
                continue;
            }
            if free && age > FREE_AGENT_AGE_LIMIT {
                continue;
            }
            if request.purpose == ScoutPurpose::Future && age > PROSPECT_AGE {
                continue;
            }
        }
        let Some(fitting) = fitting(candidate, position, role_id)? else {
            continue;
        };
        if fitting.fit == 0 {
            continue;
        }
        let owner = candidate
            .contract_club_uid
            .filter(|uid| Some(*uid) != candidate.club_uid);
        fitting_players.push((candidate.clone(), fitting, owner));
    }
    let candidates_count = fitting_players.len();

    let purpose_fits = |candidate: &ScoutCandidate, fitting: &ScoutFitting| match request.purpose {
        ScoutPurpose::FirstTeam => fitting.fit > best_own,
        ScoutPurpose::Rotation => fitting.fit >= second_own.max(best_own.saturating_sub(10)),
        ScoutPurpose::Future => room_to_grow(candidate) >= 15,
        ScoutPurpose::Any => true,
    };
    fitting_players.retain(|(candidate, fitting, _)| purpose_fits(candidate, fitting));
    let eligible = fitting_players.len();
    fitting_players.sort_by(|left, right| {
        let left_rank = f32::from(left.1.fit)
            + if request.purpose == ScoutPurpose::Future {
                potential(&left.0)
            } else {
                0.0
            };
        let right_rank = f32::from(right.1.fit)
            + if request.purpose == ScoutPurpose::Future {
                potential(&right.0)
            } else {
                0.0
            };
        right_rank
            .total_cmp(&left_rank)
            .then_with(|| right.0.ca.cmp(&left.0.ca))
    });
    fitting_players.truncate(SHORTLIST);

    let today = context.game_date;
    let mut suggestions: Vec<ScoutSuggestion> = fitting_players
        .into_iter()
        .map(|(candidate, fitting, owner)| {
            let (probability, reasons, fee, wage) = probability(&candidate, context, today);
            let owner_uid = owner.filter(|uid| Some(*uid) != candidate.club_uid);
            ScoutSuggestion {
                score: purpose_score(request.purpose, fitting.fit, probability, &candidate),
                fitting,
                probability,
                reasons,
                fee,
                wage,
                value: known_value(&candidate),
                owner_club_uid: owner_uid,
                owner_club: owner_uid.and_then(|_| candidate.contract_club_name.clone()),
            }
        })
        .filter(|suggestion| {
            request
                .max_fee
                .is_none_or(|max| suggestion.fee.is_some_and(|fee| fee <= max))
        })
        .filter(|suggestion| {
            request
                .min_probability
                .is_none_or(|min| suggestion.probability >= min)
        })
        .collect();
    suggestions.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| right.fitting.fit.cmp(&left.fitting.fit))
            .then_with(|| right.fitting.player.ca.cmp(&left.fitting.player.ca))
    });
    suggestions.truncate(
        request
            .how_many
            .unwrap_or(DEFAULT_LIMIT)
            .clamp(1, MAX_LIMIT),
    );

    Ok(Scouted {
        suggestions,
        squad,
        transfer_budget: context.transfer_budget,
        wage_budget: context.wage_budget,
        wage_bill: context.wage_bill,
        wage_room: context.wage_room(),
        examined,
        candidates: candidates_count,
        eligible,
        game_date: today.map(|date| date.format("%Y-%m-%d").to_string()),
    })
}

/// The line of the position a player plays best; none when he has no position.
fn line_of(positions: &Positions) -> Option<(usize, u8)> {
    let mut best: Option<(usize, u8)> = None;
    for (index, line) in LINES.iter().enumerate() {
        let rating = line
            .positions
            .iter()
            .filter_map(|key| positions.get(key))
            .max()
            .unwrap_or(0);
        if rating > best.map_or(0, |(_, value)| value) {
            best = Some((index, rating));
        }
    }
    best.filter(|(_, rating)| *rating > 0)
}

fn rating_in(positions: &Positions, line: usize) -> u8 {
    LINES[line]
        .positions
        .iter()
        .filter_map(|key| positions.get(key))
        .max()
        .unwrap_or(0)
}

fn line_status(players: usize, cover: usize, target: usize) -> DepthStatus {
    if players == 0 && cover == 0 {
        DepthStatus::Missing
    } else if players + cover / 2 < target {
        DepthStatus::Thin
    } else if players >= target + 2 {
        DepthStatus::Crowded
    } else {
        DepthStatus::Fine
    }
}

fn starters(target: usize) -> usize {
    target.div_ceil(2)
}

struct Member<'a> {
    candidate: &'a ScoutCandidate,
    line: usize,
    rating: u8,
    on_loan: bool,
}

/// The senior squad line by line, and the players the scout would let go.
pub fn overview(context: &ScoutContext, squad: &[ScoutCandidate]) -> ScoutOverview {
    let members: Vec<Member<'_>> = squad
        .iter()
        .filter_map(|candidate| {
            let (line, rating) = line_of(&candidate.positions)?;
            Some(Member {
                candidate,
                line,
                rating,
                on_loan: candidate.on_loan && candidate.club_uid == Some(context.club_uid),
            })
        })
        .collect();

    let mut abilities: Vec<u16> = squad.iter().map(|candidate| candidate.ca).collect();
    abilities.sort_unstable_by(|left, right| right.cmp(left));
    let level = if abilities.is_empty() {
        0
    } else {
        let count = abilities.len().min(11);
        (abilities[..count]
            .iter()
            .map(|ability| u32::from(*ability))
            .sum::<u32>()
            / count as u32) as u16
    };
    let ages: Vec<f32> = squad
        .iter()
        .filter_map(|candidate| candidate.age.map(|age| age as f32))
        .collect();
    let average_age =
        (!ages.is_empty()).then(|| round_one(ages.iter().sum::<f32>() / ages.len() as f32));

    let by_ability = |left: &ScoutDepthPlayer, right: &ScoutDepthPlayer| {
        right
            .player
            .ca
            .cmp(&left.player.ca)
            .then_with(|| right.rating.cmp(&left.rating))
    };
    let lines: Vec<ScoutLine> = LINES
        .iter()
        .enumerate()
        .map(|(index, line)| {
            let depth = |member: &Member<'_>, rating: u8| ScoutDepthPlayer {
                player: player_of(member.candidate),
                rating,
                on_loan: member.on_loan,
            };
            let mut players: Vec<ScoutDepthPlayer> = members
                .iter()
                .filter(|member| member.line == index)
                .map(|member| depth(member, member.rating))
                .collect();
            players.sort_by(by_ability);
            let mut cover: Vec<ScoutDepthPlayer> = members
                .iter()
                .filter(|member| member.line != index)
                .filter_map(|member| {
                    let rating = rating_in(&member.candidate.positions, index);
                    (rating >= 15).then(|| depth(member, rating))
                })
                .collect();
            cover.sort_by(by_ability);
            let weak = players
                .first()
                .is_some_and(|best| best.player.ca + 15 < level);
            let ageing = !players.is_empty()
                && players
                    .iter()
                    .all(|player| player.player.age.is_some_and(|age| age >= 31));
            ScoutLine {
                key: line.key,
                positions: line.positions.to_vec(),
                target: line.target,
                status: line_status(players.len(), cover.len(), line.target),
                players,
                cover,
                weak,
                ageing,
            }
        })
        .collect();

    let departures = departures(&members, &lines, level, context.game_date);
    ScoutOverview {
        club_uid: context.club_uid,
        club_name: context.club_name.clone(),
        transfer_budget: context.transfer_budget,
        wage_budget: context.wage_budget,
        wage_bill: context.wage_bill,
        wage_room: context.wage_room(),
        squad_size: squad.len(),
        average_age,
        level,
        lines,
        departures,
        positions: SCOUT_POSITION_KEYS.to_vec(),
        roles: scout_roles(),
    }
}

fn departures(
    members: &[Member<'_>],
    lines: &[ScoutLine],
    level: u16,
    today: Option<NaiveDate>,
) -> Vec<ScoutDeparture> {
    let mut wages: Vec<u32> = members
        .iter()
        .filter_map(|member| member.candidate.wage)
        .collect();
    wages.sort_unstable_by(|left, right| right.cmp(left));
    let high_wage = wages.get(wages.len() / 4).copied();

    let mut out = Vec::new();
    for member in members {
        if member.on_loan {
            continue;
        }
        let candidate = member.candidate;
        let line = &lines[member.line];
        let rank = line
            .players
            .iter()
            .position(|player| player.player.uid == candidate.uid)
            .map_or(line.players.len() + 1, |index| index + 1);
        let age = candidate.age.unwrap_or(25);
        let room = room_to_grow(candidate);
        let standing = candidate.squad_status.and_then(Standing::of);
        let first_choice = rank <= starters(line.target)
            || matches!(standing, Some(Standing::Key | Standing::Regular));

        let mut reasons = Vec::new();
        let mut priority = 0u16;
        let mut add = |code: &'static str, weight: u16, value: Option<i64>| {
            reasons.push(DepartureReason { code, value });
            priority += weight;
        };
        if line.status == DepthStatus::Crowded && rank > line.target {
            add("surplus", 3, Some(line.players.len() as i64));
        }
        let below = i64::from(candidate.ca) + 25 < i64::from(level) && age >= 23;
        if below {
            add("belowLevel", 3, Some(i64::from(candidate.ca)));
        }
        let ageing = (age >= 32 && !first_choice) || age >= 34;
        if ageing {
            add("ageing", 2, Some(i64::from(age)));
        }
        if let Some(days) = today
            .zip(candidate.contract_end)
            .map(|(today, end)| days_between(today, end))
            && (0..=365).contains(&days)
            && !first_choice
        {
            add("expiring", 2, Some(days));
        }
        if let (Some(wage), Some(high)) = (candidate.wage, high_wage)
            && wage >= high
            && !first_choice
            && i64::from(candidate.ca) < i64::from(level)
        {
            add("highWage", 2, Some(i64::from(wage)));
        }
        let blocked = age <= PROSPECT_AGE && room >= 15 && !first_choice && rank > line.target;
        if blocked {
            add("blocked", 2, Some(room));
        }
        if standing == Some(Standing::Surplus) {
            add("unwanted", 3, None);
        }
        if candidate.transfer_listed {
            add("listed", 1, None);
        }
        if reasons.is_empty() {
            continue;
        }

        let value = known_value(candidate);
        let worth_little = value.is_none_or(|value| value < 250_000);
        let action = if blocked && !below {
            DepartureAction::Loan
        } else if worth_little && age >= 28 && (below || ageing) {
            DepartureAction::Release
        } else {
            DepartureAction::Sell
        };
        out.push(ScoutDeparture {
            player: player_of(candidate),
            action,
            reasons,
            priority,
            value,
            not_for_sale: not_for_sale(candidate),
            wage: candidate.wage,
            line: line.key,
            rank,
        });
    }
    out.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| left.player.ca.cmp(&right.player.ca))
    });
    out
}

fn round_one(value: f32) -> f64 {
    (f64::from(value) * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Personality;

    fn positions(pairs: &[(&str, u8)]) -> Positions {
        let mut values = [0u8; 15];
        for (key, rating) in pairs {
            if let Some(index) = POSITION_KEYS.iter().position(|candidate| candidate == key) {
                values[index] = *rating;
            }
        }
        Positions { values }
    }

    fn attributes(value: u8) -> Attributes {
        Attributes {
            values: [value; 52],
        }
    }

    fn candidate(uid: u32, ca: u16, pa: i16, positive: &[(&str, u8)]) -> ScoutCandidate {
        ScoutCandidate {
            uid,
            name: Some(format!("Player {uid}")),
            age: Some(24),
            club_uid: Some(2),
            club_name: Some("Seller".to_owned()),
            contract_club_uid: Some(2),
            contract_club_name: Some("Seller".to_owned()),
            gender: Some(0),
            ca,
            pa: Some(pa),
            attributes: attributes(15),
            positions: positions(positive),
            personality: Some(Personality {
                ambition: 12,
                ..Personality::default()
            }),
            transfer_value: Some(10_000_000),
            transfer_value_state: TransferValueState::Ok,
            transfer_listed: false,
            wage: Some(10_000),
            contract_start: None,
            contract_end: NaiveDate::from_ymd_opt(2028, 6, 30),
            squad_status: Some(4),
            clauses: Vec::new(),
            on_loan: false,
            loan_parent_club_uid: None,
            loan_parent_club_name: None,
            reputation: Some(3_000),
            club_reputation: Some(3_000),
        }
    }

    fn context() -> ScoutContext {
        ScoutContext {
            club_uid: 1,
            club_name: "Buyer".to_owned(),
            club_gender: Some(0),
            club_reputation: Some(3_000),
            transfer_budget: Some(20_000_000),
            wage_budget: Some(500_000),
            wage_bill: Some(400_000),
            game_date: NaiveDate::from_ymd_opt(2027, 7, 1),
        }
    }

    #[test]
    fn position_familiarity_holds_fit_back() {
        let strong = fit_of(17.0, 20);
        let unfamiliar = fit_of(17.0, 11);
        assert_eq!(strong, 100);
        assert_eq!(unfamiliar, 75);
        assert_eq!(fit_of(5.0, 20), 0);
    }

    #[test]
    fn a_request_needs_a_scout_position_and_a_matching_role() {
        let request = ScoutRequest {
            position: "sw".to_owned(),
            ..ScoutRequest::default()
        };
        assert!(advise(&request, &context(), &[], &[]).is_err());

        let request = ScoutRequest {
            position: "stc".to_owned(),
            role_id: Some("goalkeeper".to_owned()),
            ..ScoutRequest::default()
        };
        assert!(advise(&request, &context(), &[], &[]).is_err());
    }

    #[test]
    fn first_team_advice_must_beat_the_best_own_player() {
        let mut own = candidate(10, 120, 140, &[("stc", 20)]);
        own.club_uid = Some(1);
        own.contract_club_uid = Some(1);
        own.attributes = attributes(14);
        let mut better = candidate(20, 140, 160, &[("stc", 20)]);
        better.club_uid = Some(2);
        let mut worse = candidate(21, 100, 120, &[("stc", 20)]);
        worse.club_uid = Some(2);
        worse.attributes = attributes(11);
        let request = ScoutRequest {
            position: "stc".to_owned(),
            purpose: ScoutPurpose::FirstTeam,
            ..ScoutRequest::default()
        };
        let scouted =
            advise(&request, &context(), &[own.clone()], &[own, better, worse]).expect("scout");
        assert_eq!(scouted.squad.len(), 1);
        assert_eq!(scouted.candidates, 2);
        assert_eq!(scouted.eligible, 1);
        assert_eq!(scouted.suggestions[0].fitting.player.uid, 20);
    }

    #[test]
    fn own_players_and_loanees_from_our_club_are_not_suggested() {
        let mut own = candidate(10, 120, 140, &[("dc", 20)]);
        own.club_uid = Some(1);
        own.contract_club_uid = Some(1);
        let mut lent_out = candidate(11, 120, 140, &[("dc", 20)]);
        lent_out.club_uid = Some(2);
        lent_out.contract_club_uid = Some(1);
        let request = ScoutRequest {
            position: "dc".to_owned(),
            purpose: ScoutPurpose::Any,
            ..ScoutRequest::default()
        };
        let scouted =
            advise(&request, &context(), &[own.clone()], &[own, lent_out]).expect("scout");
        assert!(scouted.suggestions.is_empty());
    }

    #[test]
    fn expiring_contracts_lower_the_fee_and_raise_the_probability() {
        let mut player = candidate(20, 140, 160, &[("stc", 20)]);
        player.contract_end = Some(NaiveDate::from_ymd_opt(2027, 12, 1).unwrap());
        let request = ScoutRequest {
            position: "stc".to_owned(),
            purpose: ScoutPurpose::Any,
            ..ScoutRequest::default()
        };
        let scouted = advise(&request, &context(), &[], &[player]).expect("scout");
        let suggestion = &scouted.suggestions[0];
        assert!(suggestion.fee.is_some_and(|fee| fee < 10_000_000));
        assert!(
            suggestion
                .reasons
                .iter()
                .any(|reason| reason.code == "expiring")
        );
    }

    #[test]
    fn a_live_release_clause_caps_the_fee() {
        let mut player = candidate(20, 140, 160, &[("stc", 20)]);
        player.contract_start = NaiveDate::from_ymd_opt(2026, 7, 1);
        player.clauses = vec![Clause {
            kind: 0,
            parameter: Some(1_000),
            value: Some(5_000_000),
        }];
        let request = ScoutRequest {
            position: "stc".to_owned(),
            purpose: ScoutPurpose::Any,
            ..ScoutRequest::default()
        };
        let scouted = advise(&request, &context(), &[], &[player]).expect("scout");
        let suggestion = &scouted.suggestions[0];
        assert_eq!(suggestion.fee, Some(5_000_000));
        assert!(
            suggestion
                .reasons
                .iter()
                .any(|reason| reason.code == "releaseClause" && reason.value == Some(5_000_000))
        );
    }

    #[test]
    fn unknown_values_stay_unknown() {
        let mut player = candidate(20, 140, 160, &[("stc", 20)]);
        player.transfer_value = None;
        player.transfer_value_state = TransferValueState::Unset;
        let request = ScoutRequest {
            position: "stc".to_owned(),
            purpose: ScoutPurpose::Any,
            ..ScoutRequest::default()
        };
        let scouted = advise(&request, &context(), &[], &[player]).expect("scout");
        let suggestion = &scouted.suggestions[0];
        assert_eq!(suggestion.fee, None);
        assert_eq!(suggestion.value, None);
    }

    #[test]
    fn depth_flags_missing_thin_and_crowded_lines() {
        let mut squad = Vec::new();
        for uid in 1..=6 {
            let mut player = candidate(uid, 120, 140, &[("dc", 20)]);
            player.club_uid = Some(1);
            player.contract_club_uid = Some(1);
            squad.push(player);
        }
        let report = overview(&context(), &squad);
        let dc = report.lines.iter().find(|line| line.key == "dc").unwrap();
        assert_eq!(dc.status, DepthStatus::Crowded);
        let gk = report.lines.iter().find(|line| line.key == "gk").unwrap();
        assert_eq!(gk.status, DepthStatus::Missing);
        assert_eq!(report.level, 120);
    }

    #[test]
    fn a_blocked_prospect_is_loaned_rather_than_sold() {
        let mut squad = Vec::new();
        for uid in 1..=5 {
            let mut player = candidate(uid, 150, 150, &[("stc", 20)]);
            player.club_uid = Some(1);
            player.contract_club_uid = Some(1);
            player.age = Some(28);
            squad.push(player);
        }
        let mut prospect = candidate(9, 100, 150, &[("stc", 20)]);
        prospect.club_uid = Some(1);
        prospect.contract_club_uid = Some(1);
        prospect.age = Some(19);
        squad.push(prospect);
        let report = overview(&context(), &squad);
        let departure = report
            .departures
            .iter()
            .find(|departure| departure.player.uid == 9)
            .expect("prospect departure");
        assert_eq!(departure.action, DepartureAction::Loan);
        assert!(
            departure
                .reasons
                .iter()
                .any(|reason| reason.code == "blocked")
        );
    }
}
