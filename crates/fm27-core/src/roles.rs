//! FM-style role suitability scores.
//!
//! Football Manager itself exposes a role rating for each player and role. The weights below
//! are a compact, readable approximation of the attributes FM weighs most for each role. They
//! are deliberately kept in one place so the model can be tuned without touching the query
//! layer or UI.

use std::sync::OnceLock;

use crate::model::{Attributes, Positions, RoleScore, attribute_index, position_index};

#[derive(Debug, Clone, Copy)]
pub struct RoleSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub group: &'static str,
    pub positions: &'static [&'static str],
    pub weights: &'static [(&'static str, f32)],
}

macro_rules! role {
    ($id:literal, $name:literal, $group:literal, $positions:expr, [$(($key:literal, $weight:expr)),* $(,)?]) => {
        RoleSpec {
            id: $id,
            name: $name,
            group: $group,
            positions: $positions,
            weights: &[$(($key, $weight)),*],
        }
    };
}

pub const ROLES: &[RoleSpec] = &[
    role!(
        "goalkeeper",
        "门将（防守）",
        "门将",
        &["gk"],
        [
            ("handling", 4.0),
            ("reflexes", 4.0),
            ("command_of_area", 3.0),
            ("one_on_ones", 3.0),
            ("aerial_reach", 3.0),
            ("kicking", 2.0),
            ("communication", 2.0),
            ("positioning", 2.0),
            ("concentration", 2.0),
            ("agility", 2.0),
            ("throwing", 1.0),
        ]
    ),
    role!(
        "sweeper_keeper",
        "清道夫门将（策应）",
        "门将",
        &["gk"],
        [
            ("kicking", 4.0),
            ("rushing_out", 4.0),
            ("handling", 3.0),
            ("reflexes", 3.0),
            ("passing", 3.0),
            ("decisions", 3.0),
            ("composure", 3.0),
            ("anticipation", 3.0),
            ("communication", 3.0),
            ("aerial_reach", 2.0),
            ("agility", 2.0),
        ]
    ),
    role!(
        "ball_playing_keeper",
        "组织型门将（防守）",
        "门将",
        &["gk"],
        [
            ("kicking", 5.0),
            ("passing", 4.0),
            ("composure", 4.0),
            ("decisions", 3.0),
            ("vision", 3.0),
            ("handling", 2.0),
            ("reflexes", 2.0),
            ("rushing_out", 2.0),
            ("communication", 2.0),
            ("technique", 2.0),
        ]
    ),
    role!(
        "central_defender",
        "中后卫（防守）",
        "后卫",
        &["dc"],
        [
            ("marking", 4.0),
            ("tackling", 4.0),
            ("heading", 4.0),
            ("positioning", 4.0),
            ("concentration", 3.0),
            ("bravery", 3.0),
            ("strength", 3.0),
            ("jumping_reach", 3.0),
            ("anticipation", 3.0),
            ("composure", 2.0),
        ]
    ),
    role!(
        "ball_playing_defender",
        "出球中卫（防守）",
        "后卫",
        &["dc"],
        [
            ("passing", 4.0),
            ("composure", 4.0),
            ("marking", 3.0),
            ("tackling", 3.0),
            ("heading", 3.0),
            ("technique", 3.0),
            ("decisions", 3.0),
            ("vision", 3.0),
            ("positioning", 3.0),
            ("anticipation", 2.0),
        ]
    ),
    role!(
        "no_nonsense_centre_back",
        "纯防守中卫",
        "后卫",
        &["dc"],
        [
            ("marking", 4.0),
            ("tackling", 4.0),
            ("heading", 4.0),
            ("strength", 4.0),
            ("bravery", 4.0),
            ("positioning", 3.0),
            ("concentration", 3.0),
            ("jumping_reach", 3.0),
        ]
    ),
    role!(
        "full_back",
        "边后卫（防守）",
        "后卫",
        &["dl", "dr"],
        [
            ("crossing", 3.0),
            ("tackling", 3.0),
            ("positioning", 3.0),
            ("stamina", 4.0),
            ("pace", 3.0),
            ("acceleration", 3.0),
            ("work_rate", 3.0),
            ("teamwork", 2.0),
            ("marking", 2.0),
            ("anticipation", 2.0),
        ]
    ),
    role!(
        "wing_back",
        "翼卫（进攻）",
        "后卫",
        &["wbl", "wbr", "dl", "dr"],
        [
            ("crossing", 4.0),
            ("dribbling", 3.0),
            ("stamina", 5.0),
            ("pace", 4.0),
            ("acceleration", 4.0),
            ("work_rate", 4.0),
            ("off_the_ball", 3.0),
            ("tackling", 2.0),
            ("positioning", 2.0),
            ("teamwork", 2.0),
        ]
    ),
    role!(
        "inverted_full_back",
        "内切边后卫（策应）",
        "后卫",
        &["dl", "dr", "wbl", "wbr"],
        [
            ("passing", 4.0),
            ("composure", 4.0),
            ("tackling", 3.0),
            ("marking", 3.0),
            ("positioning", 3.0),
            ("decisions", 3.0),
            ("teamwork", 3.0),
            ("work_rate", 3.0),
            ("stamina", 3.0),
            ("technique", 2.0),
        ]
    ),
    role!(
        "defensive_midfielder",
        "防守型中场（防守）",
        "中场",
        &["dm", "mc"],
        [
            ("tackling", 4.0),
            ("marking", 3.0),
            ("positioning", 4.0),
            ("anticipation", 4.0),
            ("concentration", 3.0),
            ("work_rate", 3.0),
            ("stamina", 3.0),
            ("teamwork", 3.0),
            ("strength", 2.0),
            ("passing", 2.0),
        ]
    ),
    role!(
        "anchor",
        "拖后防守（防守）",
        "中场",
        &["dm"],
        [
            ("positioning", 5.0),
            ("anticipation", 4.0),
            ("concentration", 4.0),
            ("tackling", 3.0),
            ("marking", 3.0),
            ("composure", 3.0),
            ("decisions", 3.0),
            ("teamwork", 3.0),
            ("strength", 2.0),
            ("passing", 2.0),
        ]
    ),
    role!(
        "ball_winning_midfielder",
        "抢球机器（防守）",
        "中场",
        &["dm", "mc"],
        [
            ("tackling", 5.0),
            ("aggression", 4.0),
            ("bravery", 4.0),
            ("work_rate", 4.0),
            ("stamina", 4.0),
            ("determination", 3.0),
            ("positioning", 3.0),
            ("teamwork", 3.0),
            ("marking", 2.0),
        ]
    ),
    role!(
        "deep_lying_playmaker",
        "后组织核心（策应）",
        "中场",
        &["dm", "mc"],
        [
            ("passing", 5.0),
            ("vision", 5.0),
            ("technique", 4.0),
            ("first_touch", 4.0),
            ("composure", 4.0),
            ("decisions", 4.0),
            ("anticipation", 3.0),
            ("teamwork", 2.0),
            ("balance", 2.0),
        ]
    ),
    role!(
        "central_midfielder",
        "中场（策应）",
        "中场",
        &["mc"],
        [
            ("passing", 4.0),
            ("tackling", 3.0),
            ("work_rate", 4.0),
            ("stamina", 4.0),
            ("teamwork", 3.0),
            ("decisions", 3.0),
            ("vision", 3.0),
            ("first_touch", 3.0),
            ("off_the_ball", 3.0),
            ("positioning", 3.0),
        ]
    ),
    role!(
        "box_to_box_midfielder",
        "全能中场（策应）",
        "中场",
        &["mc"],
        [
            ("stamina", 5.0),
            ("work_rate", 4.0),
            ("passing", 3.0),
            ("tackling", 3.0),
            ("off_the_ball", 3.0),
            ("teamwork", 3.0),
            ("determination", 3.0),
            ("strength", 2.0),
            ("acceleration", 2.0),
            ("finishing", 2.0),
        ]
    ),
    role!(
        "mezzala",
        "伪边锋（进攻）",
        "中场",
        &["mc", "amc"],
        [
            ("dribbling", 4.0),
            ("passing", 4.0),
            ("technique", 4.0),
            ("first_touch", 4.0),
            ("vision", 3.0),
            ("off_the_ball", 3.0),
            ("acceleration", 3.0),
            ("flair", 3.0),
            ("work_rate", 2.0),
            ("decisions", 2.0),
        ]
    ),
    role!(
        "advanced_playmaker",
        "前组织核心（进攻）",
        "中场",
        &["mc", "amc"],
        [
            ("passing", 5.0),
            ("vision", 5.0),
            ("technique", 4.0),
            ("first_touch", 4.0),
            ("decisions", 4.0),
            ("composure", 4.0),
            ("flair", 3.0),
            ("teamwork", 2.0),
            ("off_the_ball", 2.0),
        ]
    ),
    role!(
        "attacking_midfielder",
        "攻击型中场（进攻）",
        "前场",
        &["amc"],
        [
            ("passing", 4.0),
            ("dribbling", 4.0),
            ("first_touch", 4.0),
            ("technique", 4.0),
            ("vision", 4.0),
            ("off_the_ball", 4.0),
            ("decisions", 3.0),
            ("finishing", 3.0),
            ("flair", 3.0),
        ]
    ),
    role!(
        "trequartista",
        "九号半（进攻）",
        "前场",
        &["amc", "stc"],
        [
            ("technique", 5.0),
            ("first_touch", 5.0),
            ("vision", 5.0),
            ("passing", 4.0),
            ("dribbling", 4.0),
            ("flair", 5.0),
            ("composure", 4.0),
            ("off_the_ball", 3.0),
            ("finishing", 3.0),
        ]
    ),
    role!(
        "shadow_striker",
        "影锋（进攻）",
        "前场",
        &["amc", "stc"],
        [
            ("finishing", 4.0),
            ("off_the_ball", 4.0),
            ("anticipation", 4.0),
            ("composure", 4.0),
            ("dribbling", 3.0),
            ("first_touch", 3.0),
            ("decisions", 3.0),
            ("acceleration", 3.0),
            ("passing", 2.0),
        ]
    ),
    role!(
        "winger",
        "边锋（进攻）",
        "前场",
        &["aml", "amr", "ml", "mr"],
        [
            ("crossing", 5.0),
            ("dribbling", 4.0),
            ("pace", 4.0),
            ("acceleration", 4.0),
            ("technique", 3.0),
            ("off_the_ball", 3.0),
            ("first_touch", 3.0),
            ("work_rate", 2.0),
            ("flair", 2.0),
        ]
    ),
    role!(
        "inverted_winger",
        "内切边锋（进攻）",
        "前场",
        &["aml", "amr"],
        [
            ("dribbling", 5.0),
            ("finishing", 4.0),
            ("first_touch", 4.0),
            ("technique", 4.0),
            ("off_the_ball", 4.0),
            ("acceleration", 4.0),
            ("pace", 3.0),
            ("flair", 3.0),
            ("passing", 2.0),
        ]
    ),
    role!(
        "inside_forward",
        "内锋（进攻）",
        "前场",
        &["aml", "amr", "stc"],
        [
            ("finishing", 5.0),
            ("dribbling", 4.0),
            ("off_the_ball", 4.0),
            ("acceleration", 4.0),
            ("pace", 3.0),
            ("first_touch", 3.0),
            ("anticipation", 3.0),
            ("composure", 3.0),
            ("technique", 3.0),
        ]
    ),
    role!(
        "advanced_forward",
        "突前前锋（进攻）",
        "前锋",
        &["stc"],
        [
            ("finishing", 5.0),
            ("off_the_ball", 4.0),
            ("acceleration", 4.0),
            ("pace", 4.0),
            ("composure", 3.0),
            ("anticipation", 3.0),
            ("dribbling", 2.0),
            ("first_touch", 2.0),
        ]
    ),
    role!(
        "poacher",
        "抢点前锋（进攻）",
        "前锋",
        &["stc"],
        [
            ("finishing", 5.0),
            ("off_the_ball", 5.0),
            ("anticipation", 4.0),
            ("acceleration", 4.0),
            ("pace", 3.0),
            ("composure", 3.0),
            ("first_touch", 2.0),
        ]
    ),
    role!(
        "complete_forward",
        "全能前锋（进攻）",
        "前锋",
        &["stc"],
        [
            ("finishing", 4.0),
            ("dribbling", 4.0),
            ("passing", 3.0),
            ("first_touch", 3.0),
            ("technique", 3.0),
            ("strength", 3.0),
            ("pace", 3.0),
            ("off_the_ball", 3.0),
            ("work_rate", 2.0),
            ("vision", 2.0),
        ]
    ),
    role!(
        "target_forward",
        "站桩中锋（进攻）",
        "前锋",
        &["stc"],
        [
            ("heading", 5.0),
            ("strength", 5.0),
            ("jumping_reach", 4.0),
            ("finishing", 3.0),
            ("first_touch", 3.0),
            ("bravery", 3.0),
            ("teamwork", 2.0),
            ("off_the_ball", 2.0),
        ]
    ),
    role!(
        "deep_lying_forward",
        "拖后前锋（策应）",
        "前锋",
        &["stc"],
        [
            ("passing", 4.0),
            ("first_touch", 4.0),
            ("off_the_ball", 4.0),
            ("technique", 3.0),
            ("vision", 3.0),
            ("composure", 3.0),
            ("finishing", 3.0),
            ("teamwork", 2.0),
        ]
    ),
    role!(
        "pressing_forward",
        "逼抢前锋（进攻）",
        "前锋",
        &["stc"],
        [
            ("work_rate", 5.0),
            ("stamina", 4.0),
            ("aggression", 3.0),
            ("determination", 3.0),
            ("off_the_ball", 3.0),
            ("finishing", 3.0),
            ("anticipation", 3.0),
            ("pace", 2.0),
            ("teamwork", 2.0),
        ]
    ),
];

struct CompiledRole {
    spec: &'static RoleSpec,
    weights: Vec<(Option<usize>, f32)>,
    total: f32,
    positions: Vec<usize>,
}

fn compiled_roles() -> &'static [CompiledRole] {
    static COMPILED: OnceLock<Vec<CompiledRole>> = OnceLock::new();
    COMPILED.get_or_init(|| {
        ROLES
            .iter()
            .map(|spec| CompiledRole {
                spec,
                weights: spec
                    .weights
                    .iter()
                    .map(|(key, weight)| (attribute_index(key), *weight))
                    .collect(),
                total: spec.weights.iter().map(|(_, weight)| *weight).sum(),
                positions: spec
                    .positions
                    .iter()
                    .filter_map(|key| position_index(key))
                    .collect(),
            })
            .collect()
    })
}

fn score_compiled(role: &CompiledRole, attributes: &Attributes, positions: &Positions) -> (u8, u8) {
    let mut weighted = 0.0f32;
    for (index, weight) in &role.weights {
        let value = index.map_or(1, |index| attributes.values[index]) as f32;
        weighted += value * weight;
    }
    let base = if role.total > 0.0 {
        weighted / role.total
    } else {
        1.0
    };
    let best = role
        .positions
        .iter()
        .map(|index| positions.values[*index])
        .max()
        .unwrap_or(1);
    let fitness = (best as f32 / 20.0).clamp(0.0, 1.0);
    let score = (base * (0.78 + 0.22 * fitness)).round().clamp(1.0, 20.0) as u8;
    (score, best)
}

fn score_uncached(spec: &RoleSpec, attributes: &Attributes, positions: &Positions) -> (u8, u8) {
    let mut weighted = 0.0f32;
    let mut total = 0.0f32;
    for (key, weight) in spec.weights {
        let value = attributes.get(key).unwrap_or(1) as f32;
        weighted += value * weight;
        total += weight;
    }
    let base = if total > 0.0 { weighted / total } else { 1.0 };
    let best = spec
        .positions
        .iter()
        .filter_map(|key| positions.get(key))
        .max()
        .unwrap_or(1);
    let fitness = (best as f32 / 20.0).clamp(0.0, 1.0);
    let score = (base * (0.78 + 0.22 * fitness)).round().clamp(1.0, 20.0) as u8;
    (score, best)
}

fn materialize(spec: &RoleSpec, score: u8, position_rating: u8) -> RoleScore {
    RoleScore {
        id: spec.id.to_owned(),
        name: spec.name.to_owned(),
        group: spec.group.to_owned(),
        score,
        position: spec
            .positions
            .first()
            .copied()
            .unwrap_or("")
            .to_ascii_uppercase(),
        position_rating,
    }
}

pub fn score_roles(attributes: &Attributes, positions: &Positions) -> Vec<RoleScore> {
    compiled_roles()
        .iter()
        .map(|role| {
            let (score, position_rating) = score_compiled(role, attributes, positions);
            materialize(role.spec, score, position_rating)
        })
        .collect()
}

pub fn best_role(attributes: &Attributes, positions: &Positions) -> Option<RoleScore> {
    let mut best: Option<(&'static RoleSpec, u8, u8)> = None;
    for role in compiled_roles() {
        let (score, position_rating) = score_compiled(role, attributes, positions);
        // `Iterator::max_by` returns the last equal maximum, so keep `>=`.
        if best.is_none_or(|(_, best_score, _)| score >= best_score) {
            best = Some((role.spec, score, position_rating));
        }
    }
    best.map(|(spec, score, position_rating)| materialize(spec, score, position_rating))
}

pub fn role_score(spec: &RoleSpec, attributes: &Attributes, positions: &Positions) -> u8 {
    if let Some(role) = compiled_roles()
        .iter()
        .find(|role| std::ptr::eq(role.spec, spec))
    {
        return score_compiled(role, attributes, positions).0;
    }
    score_uncached(spec, attributes, positions).0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_role_scores_inside_the_display_range() {
        let attributes = Attributes { values: [15; 52] };
        let positions = Positions { values: [20; 15] };
        let scores = score_roles(&attributes, &positions);
        assert_eq!(scores.len(), ROLES.len());
        assert!(scores.iter().all(|role| (1..=20).contains(&role.score)));
        assert!(best_role(&attributes, &positions).is_some());
    }

    #[test]
    fn compiled_role_scores_match_the_string_lookup_path() {
        let mut attributes = Attributes { values: [1; 52] };
        for (index, value) in attributes.values.iter_mut().enumerate() {
            *value = (index % 20 + 1) as u8;
        }
        let mut positions = Positions { values: [1; 15] };
        for (index, value) in positions.values.iter_mut().enumerate() {
            *value = (index % 20 + 1) as u8;
        }

        for spec in ROLES {
            assert_eq!(
                role_score(spec, &attributes, &positions),
                score_uncached(spec, &attributes, &positions).0,
                "role {} changed",
                spec.id
            );
        }
    }

    #[test]
    fn best_role_matches_max_by_including_ties() {
        let attributes = Attributes { values: [10; 52] };
        let positions = Positions { values: [10; 15] };
        let expected = score_roles(&attributes, &positions)
            .into_iter()
            .max_by(|left, right| left.score.cmp(&right.score))
            .expect("expected role");
        let actual = best_role(&attributes, &positions).expect("actual role");
        assert_eq!(actual.id, expected.id);
        assert_eq!(actual.score, expected.score);
        assert_eq!(actual.position_rating, expected.position_rating);
    }

    #[test]
    fn every_role_weight_and_position_is_known() {
        for spec in ROLES {
            for (key, _) in spec.weights {
                assert!(
                    crate::model::attribute_index(key).is_some(),
                    "unknown attribute {key} in role {}",
                    spec.id
                );
            }
            for key in spec.positions {
                assert!(
                    crate::model::position_index(key).is_some(),
                    "unknown position {key} in role {}",
                    spec.id
                );
            }
        }
    }

    #[test]
    fn a_striker_is_not_a_goalkeeper() {
        let mut attributes = Attributes { values: [10; 52] };
        for key in ["finishing", "off_the_ball", "acceleration", "pace"] {
            if let Some(index) = crate::layout::ATTRIBUTE_KEYS
                .iter()
                .position(|name| *name == key)
            {
                attributes.values[index] = 20;
            }
        }
        for key in ["handling", "reflexes", "aerial_reach", "command_of_area"] {
            if let Some(index) = crate::layout::ATTRIBUTE_KEYS
                .iter()
                .position(|name| *name == key)
            {
                attributes.values[index] = 1;
            }
        }
        let positions = Positions {
            values: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 20, 1, 1],
        };
        let best = best_role(&attributes, &positions).expect("best role");
        assert!(matches!(
            best.id.as_str(),
            "advanced_forward" | "poacher" | "inside_forward" | "complete_forward"
        ));
    }
}
