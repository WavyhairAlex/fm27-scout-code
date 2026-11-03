#!/usr/bin/env python3
"""Phase 0 validation for the statistical Moneyball model.

This is a disposable analysis spike, not product code. It reads real FM27 saves through
the experimental fmsave reference implementation, checks the season-statistics fields, and
produces a first position-aware production ranking for manual review.

Usage:
    .venv/bin/python scripts/moneyball_phase0.py --out /tmp/moneyball-phase0

The default saves are the three local Parma career snapshots. Override with --save
label=path (repeatable).
"""

from __future__ import annotations

import argparse
import bisect
import csv
import json
import math
import sqlite3
import statistics
import sys
import warnings
from collections import Counter, defaultdict
from dataclasses import dataclass, fields
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable

import fmsave
from fmsave.models.season_stats import PlayerSeasonStats, SeasonStatsKind

warnings.filterwarnings("ignore", category=fmsave.UnknownBuildWarning)

GAMES_DIR = (
    Path.home()
    / "Library/Application Support/Sports Interactive/Football Manager 27/games"
)
DEFAULT_SAVES = {
    "base": GAMES_DIR / "BB KING - Parma Calcio.fm",
    "v02": GAMES_DIR / "BB KING - Parma Calcio (v02).fm",
    "v03": GAMES_DIR / "BB KING - Parma Calcio (v03).fm",
}
DEFAULT_LEAGUE_DB = (
    Path.home() / "Library/Application Support/com.fm27scout.desktop/fm27scout.db"
)

POSITION_GROUP = {
    "GK": "GK",
    "SW": "DC",
    "DC": "DC",
    "DL": "FB",
    "DR": "FB",
    "WBL": "FB",
    "WBR": "FB",
    "DM": "MID",
    "MC": "MID",
    "ML": "MID",
    "MR": "MID",
    "AML": "AM",
    "AMC": "AM",
    "AMR": "AM",
    "STC": "ST",
}
POSITION_CODES = tuple(POSITION_GROUP)
GROUP_ORDER = ("GK", "DC", "FB", "MID", "AM", "ST", "OTHER")


@dataclass(frozen=True)
class Metric:
    key: str
    attr: str
    label: str
    weight: float
    inverse: bool = False
    advanced: bool = False


# The full metric set. Advanced fields are only recorded for the roughly 63% of players
# whose row carries distance data; see metric_value().
METRICS: dict[str, tuple[Metric, ...]] = {
    "ST": (
        Metric("xg90", "expected_goals_per_90", "xG/90", 0.30),
        Metric("xa90", "expected_assists_per_90", "xA/90", 0.20),
        Metric("kp90", "key_passes_per_90", "Key passes/90", 0.15, advanced=True),
        Metric("sot90", "shots_on_target_per_90", "Shots on target/90", 0.20),
        Metric("conv", "conversion_percent", "Conversion %", 0.15),
    ),
    "AM": (
        Metric("xg90", "expected_goals_per_90", "xG/90", 0.20),
        Metric("xa90", "expected_assists_per_90", "xA/90", 0.25),
        Metric("kp90", "key_passes_per_90", "Key passes/90", 0.20, advanced=True),
        Metric("sot90", "shots_on_target_per_90", "Shots on target/90", 0.15),
        Metric("drib90", "dribbles_per_90", "Dribbles/90", 0.20, advanced=True),
    ),
    "MID": (
        Metric("xa90", "expected_assists_per_90", "xA/90", 0.20),
        Metric("kp90", "key_passes_per_90", "Key passes/90", 0.15, advanced=True),
        Metric("prog90", "progressive_passes_per_90", "Progressive passes/90", 0.20, advanced=True),
        Metric("passpct", "pass_completion_percent", "Pass completion %", 0.15),
        Metric("tack90", "tackles_completed_per_90", "Tackles won/90", 0.15),
        Metric("int90", "interceptions_per_90", "Interceptions/90", 0.15, advanced=True),
    ),
    "DC": (
        Metric("tack90", "tackles_completed_per_90", "Tackles won/90", 0.20),
        Metric("int90", "interceptions_per_90", "Interceptions/90", 0.20, advanced=True),
        Metric("headpct", "headers_won_percent", "Headers won %", 0.20, advanced=True),
        Metric("clr90", "clearances_per_90", "Clearances/90", 0.15, advanced=True),
        Metric("blk90", "blocks_per_90", "Blocks/90", 0.10, advanced=True),
        Metric("passpct", "pass_completion_percent", "Pass completion %", 0.15),
    ),
    "FB": (
        Metric("tack90", "tackles_completed_per_90", "Tackles won/90", 0.20),
        Metric("int90", "interceptions_per_90", "Interceptions/90", 0.15, advanced=True),
        Metric("headpct", "headers_won_percent", "Headers won %", 0.10, advanced=True),
        Metric("clr90", "clearances_per_90", "Clearances/90", 0.10, advanced=True),
        Metric("kp90", "key_passes_per_90", "Key passes/90", 0.15, advanced=True),
        Metric("cross90", "crosses_completed_per_90", "Crosses completed/90", 0.10, advanced=True),
        Metric("passpct", "pass_completion_percent", "Pass completion %", 0.20),
    ),
    "GK": (
        Metric("savepct", "save_percent", "Save %", 0.45, advanced=True),
        Metric("xgp90", "expected_goals_prevented_per_90", "xGP/90", 0.35, advanced=True),
        Metric(
            "ga90",
            "goals_allowed_per_90",
            "Goals allowed/90",
            0.20,
            inverse=True,
        ),
    ),
}


# The core metric set only uses fields that are present for every tracked player. This is the
# safe model for a v1 ranking: xG/xA, shots, passing and tackles are recorded across the save,
# while key passes, progressive passes, interceptions, headers, clearances, blocks, distance and
# sprints are zeroed out for about 37% of players (see the distance sentinel in metric_value).
CORE_METRICS: dict[str, tuple[Metric, ...]] = {
    "ST": (
        Metric("xg90", "expected_goals_per_90", "xG/90", 0.35),
        Metric("xa90", "expected_assists_per_90", "xA/90", 0.25),
        Metric("sot90", "shots_on_target_per_90", "Shots on target/90", 0.20),
        Metric("conv", "conversion_percent", "Conversion %", 0.20),
    ),
    "AM": (
        Metric("xg90", "expected_goals_per_90", "xG/90", 0.30),
        Metric("xa90", "expected_assists_per_90", "xA/90", 0.35),
        Metric("sot90", "shots_on_target_per_90", "Shots on target/90", 0.20),
        Metric("passpct", "pass_completion_percent", "Pass completion %", 0.15),
    ),
    "MID": (
        Metric("xa90", "expected_assists_per_90", "xA/90", 0.30),
        Metric("passpct", "pass_completion_percent", "Pass completion %", 0.25),
        Metric("pass90", "passes_completed_per_90", "Passes completed/90", 0.20),
        Metric("tack90", "tackles_completed_per_90", "Tackles won/90", 0.25),
    ),
    "DC": (
        Metric("tack90", "tackles_completed_per_90", "Tackles won/90", 0.40),
        Metric("tacklepct", "tackle_completion_percent", "Tackle completion %", 0.25),
        Metric("passpct", "pass_completion_percent", "Pass completion %", 0.35),
    ),
    "FB": (
        Metric("tack90", "tackles_completed_per_90", "Tackles won/90", 0.35),
        Metric("tacklepct", "tackle_completion_percent", "Tackle completion %", 0.20),
        Metric("passpct", "pass_completion_percent", "Pass completion %", 0.30),
        Metric("xa90", "expected_assists_per_90", "xA/90", 0.15),
    ),
    "GK": (
        Metric("ga90", "goals_allowed_per_90", "Goals allowed/90", 0.50, inverse=True),
        Metric("cs90", "clean_sheets_per_90", "Clean sheets/90", 0.30),
        Metric("rating", "average_rating", "Average rating", 0.20),
    ),
}

MODEL_METRICS = {"advanced": METRICS, "core": CORE_METRICS}


# Either 0-10000 club reputation buckets, or the value from the save cache.
REP_TIERS = (
    (2000, "1 <2k"),
    (4000, "2 2-4k"),
    (6000, "3 4-6k"),
    (8000, "4 6-8k"),
    (10_001, "5 8k+"),
)


def rep_tier(reputation: int | None) -> str:
    if reputation is None or reputation <= 0:
        return "unknown"
    for upper, label in REP_TIERS:
        if reputation < upper:
            return label
    return "5 8k+"


def position_group(player: Any) -> str:
    natural = tuple(player.natural_positions or ())
    if natural and natural[0] in POSITION_GROUP:
        return POSITION_GROUP[natural[0]]
    ratings = []
    for code in POSITION_CODES:
        value = getattr(player.positions, code.lower(), 0)
        ratings.append((value, code))
    if not ratings:
        return "OTHER"
    value, code = max(ratings)
    if value >= 15:
        return POSITION_GROUP[code]
    return "OTHER"


def metric_value(row: PlayerSeasonStats, metric: Metric) -> float | None:
    if metric.key == "cs90":
        if row.minutes <= 0:
            return None
        return row.clean_sheets * 90.0 / row.minutes
    value = getattr(row, metric.attr)
    if value is None or not math.isfinite(value):
        return None
    # Some competitions do not record the advanced statistic block at all. In those rows
    # distance_km is exactly 0 while xG/xA/passes/tackles are still present, and every
    # advanced field (key passes, progressive passes, interceptions, headers, clearances,
    # blocks, sprints, possession, saves, xGP) is zeroed out. Treat that zero as missing,
    # never as a real zero: 37% of players with 900+ minutes carry this pattern.
    if metric.advanced and row.distance_km <= 0:
        return None
    # A save percentage above 100 is impossible. Twelve low-minute cup/non-competitive rows
    # in the base save count more saves than shots faced; treat those as missing rather than
    # letting them into a goalkeeper ranking.
    if metric.key == "savepct" and value > 100:
        return None
    # xGP is stored as 0 on goalkeeper rows whose save data is unavailable. A zero there is a
    # placeholder, not an average goalkeeper.
    if metric.key == "xgp90" and row.save_percent is None:
        return None
    return float(value)


def percentile_rank(sorted_values: list[float], value: float) -> float:
    if not sorted_values:
        return 50.0
    left = bisect.bisect_left(sorted_values, value)
    right = bisect.bisect_right(sorted_values, value)
    return 100.0 * (left + 0.5 * (right - left)) / len(sorted_values)


def mean(values: Iterable[float]) -> float | None:
    values = list(values)
    return sum(values) / len(values) if values else None


def shrink(value: float, peer_mean: float, minutes: int, k: float) -> float:
    return (value * minutes + peer_mean * k) / (minutes + k)


def quantile(values: list[float], q: float) -> float:
    if not values:
        return 0.0
    values = sorted(values)
    index = (len(values) - 1) * q
    lower = math.floor(index)
    upper = math.ceil(index)
    if lower == upper:
        return values[lower]
    return values[lower] * (upper - index) + values[upper] * (index - lower)


def load_league_names(db_path: Path, save_name: str) -> dict[int, str]:
    if not db_path.exists():
        return {}
    try:
        con = sqlite3.connect(db_path)
        row = con.execute(
            "select id from saves where file_name = ? order by parsed_at desc limit 1",
            (save_name,),
        ).fetchone()
        if not row:
            return {}
        save_id = row[0]
        rows = con.execute(
            "select uid, league_name from players "
            "where save_id = ? and league_name is not null and league_name <> ''",
            (save_id,),
        ).fetchall()
        return {uid: name for uid, name in rows}
    except sqlite3.Error:
        return {}
    finally:
        try:
            con.close()
        except Exception:
            pass


@dataclass
class PlayerInfo:
    uid: int
    name: str
    age: int | None
    ca: int
    pa: int | None
    group: str
    natural_positions: tuple[str, ...]
    club_uid: int | None
    club_name: str | None
    club_reputation: int | None
    club_nation_id: int | None
    transfer_value: int | None
    transfer_value_state: str
    wage: int | None
    contract_end: str | None


def load_players(table: Any) -> dict[int, PlayerInfo]:
    players: dict[int, PlayerInfo] = {}
    for player in table:
        players[player.uid] = PlayerInfo(
            uid=player.uid,
            name=player.name or player.full_name or f"uid {player.uid}",
            age=player.age,
            ca=player.ability.current,
            pa=player.ability.potential,
            group=position_group(player),
            natural_positions=tuple(player.natural_positions or ()),
            club_uid=player.club_uid,
            club_name=player.club_name,
            club_reputation=player.club_reputation,
            club_nation_id=player.club_nation_id,
            transfer_value=player.transfer_value,
            transfer_value_state=str(player.transfer_value_state),
            wage=player.contract.wage if player.contract else None,
            contract_end=str(player.contract.end) if player.contract and player.contract.end else None,
        )
    return players


def coverage(rows: list[PlayerSeasonStats], fields_to_check: Iterable[str]) -> dict[str, dict[str, Any]]:
    total = len(rows)
    result: dict[str, dict[str, Any]] = {}
    for name in fields_to_check:
        values = [getattr(row, name) for row in rows]
        non_null = sum(value is not None for value in values)
        result[name] = {
            "non_null": non_null,
            "total": total,
            "pct": round(100.0 * non_null / total, 2) if total else 0.0,
        }
    return result


def integrity_checks(
    save_label: str,
    players: dict[int, PlayerInfo],
    clubs: Any,
    stats: list[PlayerSeasonStats],
) -> dict[str, Any]:
    club_uids = {club.uid for club in clubs}
    team_to_club: dict[int, int | None] = {}
    for club in clubs:
        for team in club.teams:
            team_to_club[team.team_id] = club.uid

    overall_rows = [row for row in stats if row.kind == SeasonStatsKind.OVERALL]
    key_counts = Counter(
        (row.player_uid, str(row.kind), row.team_id or 0) for row in stats
    )
    duplicate_keys = [
        {"key": list(key), "count": count}
        for key, count in key_counts.items()
        if count > 1
    ][:10]

    overall_by_player = Counter(row.player_uid for row in overall_rows)
    multiple_overall = [
        {"player_uid": uid, "count": count}
        for uid, count in overall_by_player.items()
        if count > 1
    ][:10]

    unknown_players = [
        row.player_uid for row in stats if row.player_uid not in players
    ]
    unknown_clubs = [
        row.club_uid for row in stats if row.club_uid is not None and row.club_uid not in club_uids
    ]
    unknown_teams = [
        row.team_id for row in stats if row.team_id is not None and row.team_id not in team_to_club
    ]

    minutes_violations = 0
    shots_violations = 0
    pass_violations = 0
    tackle_violations = 0
    header_violations = 0
    save_violations = 0
    negative_values = 0
    rating_out_of_range = 0
    gk_field_mismatch = 0
    outfield_field_mismatch = 0
    for row in stats:
        appearances = row.starts + row.substitute_appearances
        if row.minutes < 0 or row.minutes > 140 * appearances + 30:
            minutes_violations += 1
        if row.shots_on_target > row.shots or row.goals > row.shots:
            shots_violations += 1
        if row.passes_completed > row.passes_attempted:
            pass_violations += 1
        if row.tackles_completed > row.tackles_attempted:
            tackle_violations += 1
        if (
            row.headers_won is not None
            and row.aerial_challenges_attempted is not None
            and row.headers_won > row.aerial_challenges_attempted
        ):
            header_violations += 1
        if (
            row.saves_held is not None
            and row.saves_parried is not None
            and row.saves_tipped is not None
            and row.shots_on_target_faced is not None
            and row.saves_held + row.saves_parried + row.saves_tipped
            > row.shots_on_target_faced
        ):
            save_violations += 1
        numeric = (
            row.minutes,
            row.starts,
            row.substitute_appearances,
            row.goals,
            row.assists,
            row.shots,
            row.shots_on_target,
            row.passes_attempted,
            row.passes_completed,
            row.progressive_passes,
            row.key_passes,
            row.tackles_attempted,
            row.tackles_completed,
            row.interceptions,
            row.yellow_cards,
            row.red_cards,
            row.fouls_made,
            row.fouls_against,
        )
        if any(value < 0 for value in numeric):
            negative_values += 1
        if row.average_rating is not None and not (1.0 <= row.average_rating <= 10.0):
            rating_out_of_range += 1
        if row.saves_held is not None and row.aerial_challenges_attempted is not None:
            gk_field_mismatch += 1
        if row.saves_held is None and row.aerial_challenges_attempted is None:
            outfield_field_mismatch += 1

    def per90_check(attr: str, count_attr: str) -> dict[str, Any]:
        mismatches = 0
        max_error = 0.0
        checked = 0
        for row in stats:
            count = getattr(row, count_attr)
            value = getattr(row, attr)
            if count is None or value is None or row.minutes <= 0:
                continue
            checked += 1
            expected = count * 90.0 / row.minutes
            error = abs(value - expected)
            max_error = max(max_error, error)
            if error > 1e-9:
                mismatches += 1
        return {"checked": checked, "mismatches": mismatches, "max_error": max_error}

    per90 = {
        attr: per90_check(attr, count)
        for attr, count in (
            ("goals_per_90", "goals"),
            ("assists_per_90", "assists"),
            ("expected_goals_per_90", "expected_goals"),
            ("expected_assists_per_90", "expected_assists"),
            ("passes_completed_per_90", "passes_completed"),
            ("tackles_completed_per_90", "tackles_completed"),
            ("interceptions_per_90", "interceptions"),
            ("key_passes_per_90", "key_passes"),
            ("clearances_per_90", "clearances"),
            ("blocks_per_90", "blocks"),
        )
    }

    # The overall row should be the sum of the league, cup and continental rows for the same
    # player and team. Report mismatches rather than assuming it.
    by_player_team_kind: dict[tuple[int, int], dict[str, PlayerSeasonStats]] = defaultdict(dict)
    for row in stats:
        if row.team_id is None:
            continue
        if row.kind in (
            SeasonStatsKind.OVERALL,
            SeasonStatsKind.LEAGUE,
            SeasonStatsKind.CUP,
            SeasonStatsKind.CONTINENTAL,
        ):
            by_player_team_kind[(row.player_uid, row.team_id)][str(row.kind)] = row

    overall_sum_checked = 0
    overall_sum_mismatches: list[dict[str, Any]] = []
    summary_fields = ("starts", "substitute_appearances", "minutes", "goals", "assists")
    for (player_uid, team_id), kinds in by_player_team_kind.items():
        overall = kinds.get("overall")
        if overall is None:
            continue
        parts = [
            kinds.get(kind)
            for kind in ("league", "cup", "continental")
            if kinds.get(kind) is not None
        ]
        if not parts:
            continue
        overall_sum_checked += 1
        diffs = {}
        for field in summary_fields:
            expected = sum(getattr(part, field) for part in parts)
            actual = getattr(overall, field)
            if actual != expected:
                diffs[field] = {"overall": actual, "sum": expected, "diff": actual - expected}
        if diffs and len(overall_sum_mismatches) < 10:
            overall_sum_mismatches.append(
                {
                    "player_uid": player_uid,
                    "team_id": team_id,
                    "player_name": overall.player_name,
                    "diffs": diffs,
                }
            )

    return {
        "save": save_label,
        "stats_rows": len(stats),
        "overall_rows": len(overall_rows),
        "duplicate_keys": duplicate_keys,
        "duplicate_key_count": sum(1 for count in key_counts.values() if count > 1),
        "multiple_overall_rows": multiple_overall,
        "multiple_overall_count": sum(1 for count in overall_by_player.values() if count > 1),
        "unknown_player_rows": len(unknown_players),
        "unknown_club_rows": len(unknown_clubs),
        "unknown_team_rows": len(unknown_teams),
        "minutes_violations": minutes_violations,
        "shots_violations": shots_violations,
        "pass_violations": pass_violations,
        "tackle_violations": tackle_violations,
        "header_violations": header_violations,
        "save_violations": save_violations,
        "negative_values": negative_values,
        "rating_out_of_range": rating_out_of_range,
        "gk_field_mismatch": gk_field_mismatch,
        "outfield_field_mismatch": outfield_field_mismatch,
        "per90": per90,
        "overall_sum_checked": overall_sum_checked,
        "overall_sum_mismatch_count": len(overall_sum_mismatches),
        "overall_sum_mismatch_examples": overall_sum_mismatches,
    }


def build_rows(
    players: dict[int, PlayerInfo],
    stats: list[PlayerSeasonStats],
    league_names: dict[int, str],
    min_minutes: int,
    shrink_k: float,
    metrics_by_group: dict[str, tuple[Metric, ...]],
    min_coverage: float,
) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    selected = [
        row
        for row in stats
        if row.kind == SeasonStatsKind.OVERALL and row.minutes >= min_minutes
    ]
    # A player can have more than one overall row: one per team, usually a first team and a
    # B/affiliate team at the same club. Keep the row for the player's current club with the
    # most minutes, so the same player cannot enter a ranking twice and a B-team stint does
    # not dilute the first-team sample.
    by_player: dict[int, list[PlayerSeasonStats]] = defaultdict(list)
    for row in selected:
        by_player[row.player_uid].append(row)
    deduped: list[PlayerSeasonStats] = []
    for player_uid, candidates in by_player.items():
        player = players.get(player_uid)
        if player is None:
            continue
        current_club = [
            row for row in candidates if row.club_uid == player.club_uid
        ]
        pool = current_club or candidates
        deduped.append(max(pool, key=lambda row: row.minutes))
    for row in deduped:
        player = players[row.player_uid]
        rows.append(
            {
                "player_uid": row.player_uid,
                "player_name": row.player_name or player.name,
                "team_id": row.team_id,
                "club_uid": row.club_uid,
                "club_name": row.club_name or player.club_name,
                "other_team_minutes": sum(
                    candidate.minutes
                    for candidate in by_player[row.player_uid]
                    if candidate.team_id != row.team_id
                ),
                "multi_team": len(by_player[row.player_uid]) > 1,
                "advanced_available": row.distance_km > 0,
                "league_name": league_names.get(row.player_uid, ""),
                "group": player.group,
                "natural_positions": list(player.natural_positions),
                "age": player.age,
                "ca": player.ca,
                "pa": player.pa,
                "club_reputation": player.club_reputation,
                "rep_tier": rep_tier(player.club_reputation),
                "transfer_value": player.transfer_value,
                "transfer_value_state": player.transfer_value_state,
                "wage": player.wage,
                "contract_end": player.contract_end,
                "minutes": row.minutes,
                "starts": row.starts,
                "substitute_appearances": row.substitute_appearances,
                "average_rating": row.average_rating,
                "goals": row.goals,
                "assists": row.assists,
                "expected_goals": row.expected_goals,
                "expected_assists": row.expected_assists,
                "shots": row.shots,
                "shots_on_target": row.shots_on_target,
                "passes_attempted": row.passes_attempted,
                "passes_completed": row.passes_completed,
                "progressive_passes": row.progressive_passes,
                "key_passes": row.key_passes,
                "tackles_attempted": row.tackles_attempted,
                "tackles_completed": row.tackles_completed,
                "interceptions": row.interceptions,
                "aerial_challenges_attempted": row.aerial_challenges_attempted,
                "headers_won": row.headers_won,
                "clearances": row.clearances,
                "blocks": row.blocks,
                "saves_held": row.saves_held,
                "saves_parried": row.saves_parried,
                "saves_tipped": row.saves_tipped,
                "shots_on_target_faced": row.shots_on_target_faced,
                "expected_goals_prevented": row.expected_goals_prevented,
                "_row": row,
            }
        )

    # Peer means: position group x reputation tier, falling back to the position group when a
    # cell is too small to be meaningful. Only scored rows (>= min_minutes) enter the peer
    # distribution; players below the threshold are shown as insufficient sample instead.
    means_by_group: dict[str, dict[str, float]] = {}
    means_by_cell: dict[tuple[str, str], dict[str, float]] = {}
    mean_base = rows
    for group in {row["group"] for row in rows}:
        group_base = [row for row in mean_base if row["group"] == group]
        means_by_group[group] = {}
        for cell_tier in {row["rep_tier"] for row in group_base}:
            cell_base = [row for row in group_base if row["rep_tier"] == cell_tier]
            means_by_cell[(group, cell_tier)] = {}
            for metric in metrics_by_group.get(group, ()):
                values = [
                    value
                    for row in cell_base
                    if (value := metric_value(row["_row"], metric)) is not None
                ]
                cell_mean = mean(values)
                if cell_mean is not None and len(values) >= 30:
                    means_by_cell[(group, cell_tier)][metric.key] = cell_mean
            for metric in metrics_by_group.get(group, ()):
                values = [
                    value
                    for row in group_base
                    if (value := metric_value(row["_row"], metric)) is not None
                ]
                group_mean = mean(values)
                if group_mean is not None:
                    means_by_group[group][metric.key] = group_mean
                if metric.key not in means_by_cell.get((group, cell_tier), {}):
                    if group_mean is not None:
                        means_by_cell.setdefault((group, cell_tier), {})[metric.key] = group_mean

    # Shrink every metric toward its peer mean.
    for row in rows:
        group = row["group"]
        cell = (group, row["rep_tier"])
        row["adjusted"] = {}
        for metric in metrics_by_group.get(group, ()):
            value = metric_value(row["_row"], metric)
            if value is None:
                continue
            peer = means_by_cell.get(cell, {}).get(
                metric.key, means_by_group.get(group, {}).get(metric.key)
            )
            if peer is None:
                continue
            row["adjusted"][metric.key] = shrink(
                value, peer, row["minutes"], shrink_k
            )

    # Percentile within the position group and within the position group x reputation tier.
    for basis in ("global", "tier"):
        buckets: dict[Any, dict[str, list[float]]] = defaultdict(lambda: defaultdict(list))
        for row in rows:
            key = row["group"] if basis == "global" else (row["group"], row["rep_tier"])
            for metric_key, value in row["adjusted"].items():
                buckets[key][metric_key].append(value)
        for bucket in buckets.values():
            for values in bucket.values():
                values.sort()
        for row in rows:
            key = row["group"] if basis == "global" else (row["group"], row["rep_tier"])
            cell = buckets.get(key, {})
            group_cell = buckets.get(row["group"], {})
            row[f"pct_{basis}"] = {}
            for metric in metrics_by_group.get(row["group"], ()):
                value = row["adjusted"].get(metric.key)
                if value is None:
                    continue
                values = cell.get(metric.key) or group_cell.get(metric.key) or []
                if not values:
                    continue
                pct = percentile_rank(values, value)
                row[f"pct_{basis}"][metric.key] = 100.0 - pct if metric.inverse else pct

    for basis in ("global", "tier"):
        for row in rows:
            metrics = metrics_by_group.get(row["group"], ())
            available = [
                (metric, row[f"pct_{basis}"].get(metric.key))
                for metric in metrics
                if row[f"pct_{basis}"].get(metric.key) is not None
            ]
            total_weight = sum(metric.weight for metric in metrics)
            available_weight = sum(metric.weight for metric, _ in available)
            if total_weight == 0 or available_weight / total_weight < min_coverage:
                row[f"production_{basis}"] = None
                row[f"coverage_{basis}"] = available_weight / total_weight if total_weight else 0
                row[f"drivers_{basis}"] = []
                continue
            score = sum(pct * metric.weight for metric, pct in available) / available_weight
            row[f"production_{basis}"] = score
            row[f"coverage_{basis}"] = available_weight / total_weight
            contributions = sorted(
                (
                    {
                        "metric": metric.key,
                        "label": metric.label,
                        "percentile": round(pct, 1),
                        "weight": metric.weight,
                        "contribution": round(pct * metric.weight / available_weight, 1),
                    }
                    for metric, pct in available
                ),
                key=lambda item: item["contribution"],
                reverse=True,
            )
            row[f"drivers_{basis}"] = contributions[:3]

    # Cost: transfer valuation plus three years of wages. Only rows with a usable fee and wage
    # enter the cost view; unknown values are never treated as zero.
    for row in rows:
        fee = row["transfer_value"]
        fee_ok = row["transfer_value_state"] == "ok" and fee is not None and fee > 0
        wage = row["wage"]
        wage_ok = wage is not None and wage > 0
        row["cost_total"] = fee + wage * 52 * 3 if fee_ok and wage_ok else None

    for basis in ("global", "tier"):
        buckets: dict[Any, list[float]] = defaultdict(list)
        for row in rows:
            if row["cost_total"] is None:
                continue
            key = row["group"] if basis == "global" else (row["group"], row["rep_tier"])
            buckets[key].append(row["cost_total"])
        for values in buckets.values():
            values.sort()
        for row in rows:
            if row["cost_total"] is None:
                row[f"cost_pct_{basis}"] = None
                continue
            key = row["group"] if basis == "global" else (row["group"], row["rep_tier"])
            values = buckets.get(key) or buckets.get(row["group"]) or []
            if not values:
                row[f"cost_pct_{basis}"] = None
                continue
            row[f"cost_pct_{basis}"] = percentile_rank(values, row["cost_total"])

    for basis in ("global", "tier"):
        for row in rows:
            production = row[f"production_{basis}"]
            cost_pct = row[f"cost_pct_{basis}"]
            row[f"surplus_{basis}"] = (
                production - cost_pct
                if production is not None and cost_pct is not None
                else None
            )
    return rows


def public_row(row: dict[str, Any], basis: str) -> dict[str, Any]:
    keys = (
        "player_uid",
        "player_name",
        "team_id",
        "multi_team",
        "other_team_minutes",
        "advanced_available",
        "club_name",
        "league_name",
        "group",
        "age",
        "ca",
        "pa",
        "club_reputation",
        "rep_tier",
        "minutes",
        "starts",
        "substitute_appearances",
        "average_rating",
        "goals",
        "assists",
        "expected_goals",
        "expected_assists",
        "transfer_value",
        "transfer_value_state",
        "wage",
        "cost_total",
        "contract_end",
    )
    result = {key: row.get(key) for key in keys}
    result["production"] = round(row[f"production_{basis}"], 2) if row[f"production_{basis}"] is not None else None
    result["coverage"] = round(row[f"coverage_{basis}"], 3)
    result["cost_pct"] = round(row[f"cost_pct_{basis}"], 1) if row[f"cost_pct_{basis}"] is not None else None
    result["surplus"] = round(row[f"surplus_{basis}"], 1) if row[f"surplus_{basis}"] is not None else None
    result["drivers"] = row[f"drivers_{basis}"]
    return result


def top_rows(rows: list[dict[str, Any]], key: str, limit: int, exclude_gk: bool = True) -> list[dict[str, Any]]:
    eligible = [
        row
        for row in rows
        if row.get(key) is not None and (not exclude_gk or row["group"] != "GK")
    ]
    return sorted(eligible, key=lambda row: row[key], reverse=True)[:limit]


def composition(rows: list[dict[str, Any]]) -> dict[str, Any]:
    if not rows:
        return {}
    groups = Counter(row["group"] for row in rows)
    tiers = Counter(row["rep_tier"] for row in rows)
    leagues = Counter(row["league_name"] or "unknown" for row in rows)
    known_leagues = Counter(
        row["league_name"] for row in rows if row["league_name"]
    )
    clubs = Counter(row["club_name"] or "unknown" for row in rows)
    ages = [row["age"] for row in rows if row["age"] is not None]
    cas = [row["ca"] for row in rows if row["ca"] is not None]
    minutes = [row["minutes"] for row in rows]
    reputations = [row["club_reputation"] for row in rows if row["club_reputation"]]
    return {
        "count": len(rows),
        "groups": dict(groups),
        "tiers": dict(tiers),
        "leagues": dict(leagues),
        "known_leagues": dict(known_leagues),
        "known_league_rows": sum(known_leagues.values()),
        "max_club_share": round(100.0 * max(clubs.values()) / len(rows), 1) if clubs else 0,
        "max_league_share": round(100.0 * max(known_leagues.values()) / len(rows), 1)
        if known_leagues
        else 0,
        "max_known_league_share": round(
            100.0 * max(known_leagues.values()) / sum(known_leagues.values()), 1
        )
        if known_leagues
        else 0,
        "top_clubs": clubs.most_common(5),
        "top_known_leagues": known_leagues.most_common(5),
        "age": {
            "median": statistics.median(ages) if ages else None,
            "min": min(ages) if ages else None,
            "max": max(ages) if ages else None,
        },
        "ca": {
            "median": statistics.median(cas) if cas else None,
            "min": min(cas) if cas else None,
            "max": max(cas) if cas else None,
        },
        "minutes": {
            "median": statistics.median(minutes) if minutes else None,
            "min": min(minutes) if minutes else None,
            "max": max(minutes) if minutes else None,
        },
        "reputation": {
            "median": statistics.median(reputations) if reputations else None,
            "min": min(reputations) if reputations else None,
            "max": max(reputations) if reputations else None,
        },
    }


SAMPLE_QUANTILES = {
    "GK": 2,
    "DC": 3,
    "FB": 3,
    "MID": 4,
    "AM": 4,
    "ST": 4,
}


def manual_sample(rows: list[dict[str, Any]], basis: str) -> list[dict[str, Any]]:
    sample: list[dict[str, Any]] = []
    for group, count in SAMPLE_QUANTILES.items():
        candidates = sorted(
            [row for row in rows if row["group"] == group and row[f"production_{basis}"] is not None],
            key=lambda row: row[f"production_{basis}"],
        )
        if not candidates:
            continue
        club_counts: Counter[str] = Counter()
        picked: list[dict[str, Any]] = []
        if count == 1:
            positions = [0.5]
        else:
            positions = [0.95 - i * (0.70 / (count - 1)) for i in range(count)]
        for q in positions:
            index = min(len(candidates) - 1, max(0, round(q * (len(candidates) - 1))))
            # Walk outward from the target quantile until a club-diverse, unused row is found.
            chosen = None
            for offset in range(len(candidates)):
                for candidate_index in (index + offset, index - offset):
                    if candidate_index < 0 or candidate_index >= len(candidates):
                        continue
                    candidate = candidates[candidate_index]
                    if candidate["player_uid"] in {row["player_uid"] for row in picked}:
                        continue
                    if club_counts[candidate["club_name"] or "unknown"] >= 2:
                        continue
                    chosen = candidate
                    break
                if chosen is not None:
                    break
            if chosen is None:
                continue
            picked.append(chosen)
            club_counts[chosen["club_name"] or "unknown"] += 1
        sample.extend(picked[:count])
    return sample


SAMPLE_COLUMNS = (
    "player_name",
    "club_name",
    "league_name",
    "group",
    "age",
    "ca",
    "pa",
    "minutes",
    "starts",
    "substitute_appearances",
    "goals",
    "assists",
    "expected_goals",
    "expected_assists",
    "shots",
    "shots_on_target",
    "passes_attempted",
    "passes_completed",
    "pass_completion_percent",
    "progressive_passes",
    "key_passes",
    "tackles_attempted",
    "tackles_completed",
    "interceptions",
    "aerial_challenges_attempted",
    "headers_won",
    "headers_won_percent",
    "clearances",
    "blocks",
    "average_rating",
    "saves_held",
    "saves_parried",
    "saves_tipped",
    "shots_on_target_faced",
    "save_percent",
    "expected_goals_prevented",
    "expected_goals_prevented_per_90",
    "manual_verified",
    "notes",
)


def sample_record(row: dict[str, Any]) -> dict[str, Any]:
    stats = row["_row"]
    result = {}
    for column in SAMPLE_COLUMNS:
        if column in ("manual_verified", "notes"):
            result[column] = ""
        elif column == "pass_completion_percent":
            result[column] = round(stats.pass_completion_percent, 2) if stats.pass_completion_percent is not None else ""
        elif column == "headers_won_percent":
            result[column] = round(stats.headers_won_percent, 2) if stats.headers_won_percent is not None else ""
        elif column == "save_percent":
            result[column] = round(stats.save_percent, 2) if stats.save_percent is not None else ""
        elif column == "expected_goals_prevented":
            result[column] = (
                round(stats.expected_goals_prevented, 3)
                if stats.saves_held is not None and stats.save_percent is not None
                else ""
            )
        elif column == "expected_goals_prevented_per_90":
            value = stats.expected_goals_prevented_per_90
            result[column] = (
                round(value, 3)
                if value is not None
                and stats.saves_held is not None
                and stats.save_percent is not None
                else ""
            )
        else:
            value = row.get(column)
            if isinstance(value, float):
                value = round(value, 3)
            result[column] = "" if value is None else value
    return result


def raw_sample_row(
    row: PlayerSeasonStats, player: PlayerInfo, league_name: str
) -> dict[str, Any]:
    return {
        "player_uid": row.player_uid,
        "player_name": row.player_name or player.name,
        "club_name": row.club_name or player.club_name,
        "league_name": league_name,
        "group": player.group,
        "age": player.age,
        "ca": player.ca,
        "pa": player.pa,
        "minutes": row.minutes,
        "starts": row.starts,
        "substitute_appearances": row.substitute_appearances,
        "goals": row.goals,
        "assists": row.assists,
        "expected_goals": row.expected_goals,
        "expected_assists": row.expected_assists,
        "shots": row.shots,
        "shots_on_target": row.shots_on_target,
        "passes_attempted": row.passes_attempted,
        "passes_completed": row.passes_completed,
        "progressive_passes": row.progressive_passes,
        "key_passes": row.key_passes,
        "tackles_attempted": row.tackles_attempted,
        "tackles_completed": row.tackles_completed,
        "interceptions": row.interceptions,
        "aerial_challenges_attempted": row.aerial_challenges_attempted,
        "headers_won": row.headers_won,
        "clearances": row.clearances,
        "blocks": row.blocks,
        "saves_held": row.saves_held,
        "saves_parried": row.saves_parried,
        "saves_tipped": row.saves_tipped,
        "shots_on_target_faced": row.shots_on_target_faced,
        "average_rating": row.average_rating,
        "_row": row,
    }


def write_csv(path: Path, rows: list[dict[str, Any]], columns: Iterable[str]) -> None:
    columns = list(columns)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=columns, extrasaction="ignore")
        writer.writeheader()
        for row in rows:
            writer.writerow({column: row.get(column, "") for column in columns})


def analyze_save(
    label: str,
    path: Path,
    league_db: Path,
    min_minutes: int,
    shrink_k: float,
    top_n: int,
    model: str,
) -> tuple[dict[str, Any], list[dict[str, Any]], dict[str, Any]]:
    with fmsave.open(path, experimental_fm27=True) as save:
        game_date = str(save.info.game_date)
        stats_table = save.player_season_stats()
        stats = list(stats_table)
        players_table = save.players()
        players = load_players(players_table)
        clubs = list(save.clubs())
        try:
            managed = list(save.managed_clubs())
        except Exception:
            managed = []
    league_names = load_league_names(league_db, path.name)

    field_names = [field.name for field in fields(PlayerSeasonStats)]
    computed_names = list(PlayerSeasonStats.COMPUTED_FIELDS)
    overall_rows = [row for row in stats if row.kind == SeasonStatsKind.OVERALL]
    overall_main = [row for row in overall_rows if row.minutes >= min_minutes]
    field_coverage = {
        "all": coverage(stats, field_names + computed_names),
        "overall": coverage(overall_rows, field_names + computed_names),
        f"overall_ge_{min_minutes}": coverage(overall_main, field_names + computed_names),
    }
    checks = integrity_checks(label, players, clubs, stats)

    rows = build_rows(
        players,
        stats,
        league_names,
        min_minutes=min_minutes,
        shrink_k=shrink_k,
        metrics_by_group=MODEL_METRICS[model],
        min_coverage=0.99 if model == "advanced" else 0.6,
    )
    production_global = top_rows(rows, "production_global", top_n)
    production_tier = top_rows(rows, "production_tier", top_n)
    surplus_global = top_rows(rows, "surplus_global", top_n)
    surplus_tier = top_rows(rows, "surplus_tier", top_n)

    def filtered_top(
        key: str,
        *,
        min_ca: int | None = None,
        max_age: int | None = None,
        min_tier: str | None = None,
    ) -> list[dict[str, Any]]:
        eligible = [
            row
            for row in rows
            if row.get(key) is not None
            and row["group"] != "GK"
            and (min_ca is None or (row["ca"] is not None and row["ca"] >= min_ca))
            and (max_age is None or (row["age"] is not None and row["age"] <= max_age))
            and (min_tier is None or row["rep_tier"] >= min_tier)
        ]
        return sorted(eligible, key=lambda row: row[key], reverse=True)[:top_n]

    constrained_surplus = filtered_top("surplus_tier", min_ca=120, max_age=30)
    constrained_production = filtered_top("production_tier", min_ca=120, max_age=30)
    reputable_surplus = filtered_top("surplus_tier", min_ca=110, min_tier="3 4-6k")
    def group_tops(
        key: str,
        *,
        min_ca: int | None = None,
        max_age: int | None = None,
    ) -> dict[str, list[dict[str, Any]]]:
        result: dict[str, list[dict[str, Any]]] = {}
        for group in GROUP_ORDER:
            if group == "OTHER":
                continue
            eligible = [
                row
                for row in rows
                if row["group"] == group
                and row.get(key) is not None
                and (min_ca is None or (row["ca"] is not None and row["ca"] >= min_ca))
                and (max_age is None or (row["age"] is not None and row["age"] <= max_age))
            ]
            result[group] = [
                public_row(row, "tier")
                for row in sorted(eligible, key=lambda row: row[key], reverse=True)[:top_n]
            ]
        return result

    by_group = group_tops("production_tier")
    by_group_constrained = group_tops("production_tier", min_ca=120, max_age=30)
    by_group_surplus_constrained = group_tops("surplus_tier", min_ca=120, max_age=30)

    league_coverage = sum(1 for row in rows if row["league_name"])
    advanced_available = sum(1 for row in rows if row["_row"].distance_km > 0)
    advanced_by_tier = Counter(
        row["rep_tier"] for row in rows if row["_row"].distance_km > 0
    )
    total_by_tier = Counter(row["rep_tier"] for row in rows)
    managed_uids = {club.club_uid for club in managed}
    managed_by_player: dict[int, PlayerSeasonStats] = {}
    for row in stats:
        if (
            row.kind == SeasonStatsKind.OVERALL
            and row.club_uid in managed_uids
            and row.minutes >= 450
        ):
            current = managed_by_player.get(row.player_uid)
            if current is None or row.minutes > current.minutes:
                managed_by_player[row.player_uid] = row
    managed_sample = [
        sample_record(
            raw_sample_row(
                row,
                players[row.player_uid],
                league_names.get(row.player_uid, ""),
            )
        )
        for row in sorted(
            managed_by_player.values(), key=lambda item: item.minutes, reverse=True
        )
        if row.player_uid in players
    ]
    summary = {
        "label": label,
        "path": str(path),
        "model": model,
        "game_date": game_date,
        "managed_clubs": [
            {
                "club_uid": club.club_uid,
                "club_name": club.club_name,
                "manager_name": club.manager_name,
            }
            for club in managed
        ],
        "managed_sample": managed_sample,
        "player_count": len(players),
        "stats_rows": len(stats),
        "stats_players": len({row.player_uid for row in stats}),
        "kinds": dict(Counter(str(row.kind) for row in stats)),
        "overall_rows": len(overall_rows),
        f"overall_ge_{min_minutes}": len(overall_main),
        "scored_rows": len(rows),
        "model_scored_rows": sum(
            1 for row in rows if row.get("production_tier") is not None
        ),
        "cost_rows": sum(1 for row in rows if row.get("cost_total") is not None),
        "league_name_coverage": {
            "rows": league_coverage,
            "total": len(rows),
            "pct": round(100.0 * league_coverage / len(rows), 2) if rows else 0.0,
        },
        "advanced_available": advanced_available,
        "advanced_by_tier": {
            tier: {
                "available": advanced_by_tier.get(tier, 0),
                "total": total_by_tier.get(tier, 0),
                "pct": round(
                    100.0 * advanced_by_tier.get(tier, 0) / total_by_tier[tier], 1
                )
                if total_by_tier.get(tier)
                else 0.0,
            }
            for tier in sorted(total_by_tier)
        },
        "field_coverage": field_coverage,
        "integrity": checks,
        "top": {
            "production_global": [public_row(row, "global") for row in production_global],
            "production_tier": [public_row(row, "tier") for row in production_tier],
            "surplus_global": [public_row(row, "global") for row in surplus_global],
            "surplus_tier": [public_row(row, "tier") for row in surplus_tier],
            "constrained_production": [
                public_row(row, "tier") for row in constrained_production
            ],
            "constrained_surplus": [
                public_row(row, "tier") for row in constrained_surplus
            ],
            "reputable_surplus": [
                public_row(row, "tier") for row in reputable_surplus
            ],
        },
        "composition": {
            "production_global": composition([public_row(row, "global") for row in production_global]),
            "production_tier": composition([public_row(row, "tier") for row in production_tier]),
            "surplus_global": composition([public_row(row, "global") for row in surplus_global]),
            "surplus_tier": composition([public_row(row, "tier") for row in surplus_tier]),
            "constrained_production": composition(
                [public_row(row, "tier") for row in constrained_production]
            ),
            "constrained_surplus": composition(
                [public_row(row, "tier") for row in constrained_surplus]
            ),
            "reputable_surplus": composition(
                [public_row(row, "tier") for row in reputable_surplus]
            ),
            "by_group": {
                group: composition(rows_for_group) for group, rows_for_group in by_group.items()
            },
        },
        "by_group": by_group,
        "by_group_constrained": by_group_constrained,
        "by_group_surplus_constrained": by_group_surplus_constrained,
        "sample": [sample_record(row) for row in manual_sample(rows, "tier")],
    }
    return summary, rows, checks


def stability(
    analyses: dict[str, list[dict[str, Any]]], top_n: int
) -> dict[str, Any]:
    labels = list(analyses)
    result: dict[str, Any] = {}
    for basis in ("global", "tier"):
        sets = {
            label: {
                row["player_uid"]
                for row in top_rows(rows, f"production_{basis}", top_n)
            }
            for label, rows in analyses.items()
        }
        pair_overlap = {}
        for i, left in enumerate(labels):
            for right in labels[i + 1 :]:
                intersection = len(sets[left] & sets[right])
                pair_overlap[f"{left}__{right}"] = {
                    "overlap": intersection,
                    "top_n": top_n,
                    "pct": round(100.0 * intersection / top_n, 1),
                }
        result[f"production_{basis}"] = pair_overlap
    return result


def write_report(
    out_dir: Path,
    summaries: dict[str, dict[str, Any]],
    stability_result: dict[str, Any],
    min_minutes: int,
    shrink_k: float,
    top_n: int,
    model: str,
) -> None:
    lines: list[str] = []
    lines.append("# Moneyball Phase 0 validation")
    lines.append("")
    lines.append(f"Generated: {datetime.now(timezone.utc).isoformat(timespec='seconds')}")
    lines.append("")
    lines.append("## Configuration")
    lines.append("")
    lines.append(f"- Model: {model}")
    lines.append(f"- Minimum minutes: {min_minutes}")
    lines.append(f"- Shrinkage K: {shrink_k}")
    lines.append(f"- Top N: {top_n}")
    lines.append("- Overall rows only; other-team rows are ignored for rankings.")
    lines.append("- Production percentiles: position group, and position group x reputation tier.")
    lines.append("- Cost: transfer_value + weekly wage x 52 x 3, only when both are usable.")
    lines.append("")

    lines.append("## Save coverage")
    lines.append("")
    lines.append("| Save | Game date | Players | Stats rows | Players with stats | Overall rows | Overall >= min | Scored | Model scored | Cost rows | League name coverage |")
    lines.append("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
    for label, summary in summaries.items():
        coverage_info = summary["league_name_coverage"]
        lines.append(
            f"| {label} | {summary['game_date']} | {summary['player_count']:,} | {summary['stats_rows']:,} | "
            f"{summary['stats_players']:,} | {summary['overall_rows']:,} | "
            f"{summary[f'overall_ge_{min_minutes}']:,} | {summary['scored_rows']:,} | "
            f"{summary['model_scored_rows']:,} | {summary['cost_rows']:,} | "
            f"{coverage_info['rows']:,}/{coverage_info['total']:,} ({coverage_info['pct']}%) |"
        )
    lines.append("")

    lines.append("## Advanced-stat availability")
    lines.append("")
    lines.append(
        "Rows with `distance_km > 0` carry the advanced block (key passes, progressive "
        "passes, interceptions, headers, clearances, blocks, sprints, possession, saves, xGP). "
        "Rows with `distance_km == 0` have that block zeroed out while xG/xA/passes/tackles "
        "remain present."
    )
    lines.append("")
    lines.append("| Save | Advanced rows | Total scored | Coverage |")
    lines.append("|---|---:|---:|---:|")
    for label, summary in summaries.items():
        lines.append(
            f"| {label} | {summary['advanced_available']:,} | {summary['scored_rows']:,} | "
            f"{100.0 * summary['advanced_available'] / summary['scored_rows']:.1f}% |"
        )
    lines.append("")
    lines.append("| Save | Rep tier | Advanced rows | Total | Coverage |")
    lines.append("|---|---|---:|---:|---:|")
    for label, summary in summaries.items():
        for tier, info in summary["advanced_by_tier"].items():
            lines.append(
                f"| {label} | {tier} | {info['available']:,} | {info['total']:,} | {info['pct']}% |"
            )
    lines.append("")

    lines.append("## Integrity checks")
    lines.append("")
    lines.append("| Save | Dup keys | Multi overall | Unknown player rows | Unknown club rows | Unknown team rows | Overall sum mismatches | Per-90 max error |")
    lines.append("|---|---:|---:|---:|---:|---:|---:|---:|")
    for label, summary in summaries.items():
        checks = summary["integrity"]
        max_error = max(
            (item["max_error"] for item in checks["per90"].values()), default=0.0
        )
        lines.append(
            f"| {label} | {checks['duplicate_key_count']} | {checks['multiple_overall_count']} | "
            f"{checks['unknown_player_rows']} | {checks['unknown_club_rows']} | "
            f"{checks['unknown_team_rows']} | "
            f"{checks['overall_sum_mismatch_count']}/{checks['overall_sum_checked']} | "
            f"{max_error:.2e} |"
        )
    lines.append("")
    lines.append("Invariant violations (minutes, shots, passes, tackles, headers, saves, negatives, rating range):")
    lines.append("")
    lines.append("| Save | Minutes | Shots | Passes | Tackles | Headers | Saves | Negatives | Rating | GK field mismatch | Outfield field mismatch |")
    lines.append("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
    for label, summary in summaries.items():
        checks = summary["integrity"]
        lines.append(
            f"| {label} | {checks['minutes_violations']} | {checks['shots_violations']} | "
            f"{checks['pass_violations']} | {checks['tackle_violations']} | "
            f"{checks['header_violations']} | {checks['save_violations']} | "
            f"{checks['negative_values']} | {checks['rating_out_of_range']} | "
            f"{checks['gk_field_mismatch']} | {checks['outfield_field_mismatch']} |"
        )
    lines.append("")

    lines.append("## Top 50 composition")
    lines.append("")
    lines.append("| Save | View | Groups | Rep tiers | Age median | CA median | Max club share | Max known-league share |")
    lines.append("|---|---|---|---|---:|---:|---:|---:|")
    for label, summary in summaries.items():
        for view in (
            "production_global",
            "production_tier",
            "surplus_global",
            "surplus_tier",
            "constrained_production",
            "constrained_surplus",
            "reputable_surplus",
        ):
            comp = summary["composition"][view]
            if not comp:
                continue
            tiers = ", ".join(f"{key}:{value}" for key, value in sorted(comp["tiers"].items()))
            groups = ", ".join(f"{key}:{value}" for key, value in sorted(comp["groups"].items()))
            lines.append(
                f"| {label} | {view} | {groups} | {tiers} | "
                f"{comp['age']['median']} | {comp['ca']['median']} | "
                f"{comp['max_club_share']}% | {comp['max_known_league_share']}% "
                f"({comp['known_league_rows']}/{comp['count']} named) |"
            )
    lines.append("")

    lines.append("## Stability across the three career snapshots")
    lines.append("")
    lines.append("| Basis | Pair | Overlap |")
    lines.append("|---|---|---:|")
    for basis, pairs in stability_result.items():
        for pair, value in pairs.items():
            lines.append(f"| {basis} | {pair} | {value['overlap']}/{value['top_n']} ({value['pct']}%) |")
    lines.append("")

    lines.append("## Manual sample")
    lines.append("")
    lines.append(
        "`sample_20.csv` contains 20 players stratified by position group and production "
        "quantile. Check each row against the in-game player profile and fill in "
        "`manual_verified` / `notes`. `sample_managed.csv` contains every player at the "
        "managed club with at least 450 minutes, which is the easiest set to check in game."
    )
    lines.append("")
    lines.append("## Go / No-Go checklist")
    lines.append("")
    lines.append("- [ ] At least 8 fields pass structural and range checks on all three saves.")
    lines.append("- [ ] 20-player manual sample matches the in-game profile (counts +/-1, xG/xA +/-0.05).")
    lines.append("- [ ] Top 50 contains no systematic lower-league bias after tiering.")
    lines.append("- [ ] Top 50 is reasonably stable across career snapshots.")
    lines.append("- [ ] No single club or league exceeds 30% of a top 50 without an explicit filter.")
    lines.append("")
    out_dir.joinpath("report.md").write_text("\n".join(lines), encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--save",
        action="append",
        default=None,
        metavar="LABEL=PATH",
        help="Save to analyse; repeatable. Defaults to the three Parma snapshots.",
    )
    parser.add_argument("--out", type=Path, default=Path("/tmp/moneyball-phase0"))
    parser.add_argument("--league-db", type=Path, default=DEFAULT_LEAGUE_DB)
    parser.add_argument("--min-minutes", type=int, default=900)
    parser.add_argument("--shrink-k", type=float, default=900.0)
    parser.add_argument("--top", type=int, default=50)
    parser.add_argument(
        "--model",
        choices=tuple(MODEL_METRICS),
        default="core",
        help="core uses only fields present for every player; advanced uses the full set "
        "but leaves the 37%% of players without the advanced block unscored.",
    )
    args = parser.parse_args()

    if args.save:
        saves: dict[str, Path] = {}
        for item in args.save:
            if "=" not in item:
                parser.error("--save must be LABEL=PATH")
            label, path = item.split("=", 1)
            saves[label] = Path(path).expanduser()
    else:
        saves = dict(DEFAULT_SAVES)
    missing = [str(path) for path in saves.values() if not path.exists()]
    if missing:
        parser.error("save not found: " + ", ".join(missing))

    args.out.mkdir(parents=True, exist_ok=True)
    summaries: dict[str, dict[str, Any]] = {}
    all_rows: dict[str, list[dict[str, Any]]] = {}
    for label, path in saves.items():
        print(f"[{label}] {path}", file=sys.stderr)
        summary, rows, _checks = analyze_save(
            label,
            path,
            args.league_db,
            args.min_minutes,
            args.shrink_k,
            args.top,
            args.model,
        )
        summaries[label] = summary
        all_rows[label] = rows
        print(
            f"  players={summary['player_count']:,} stats={summary['stats_rows']:,} "
            f"scored={summary['scored_rows']:,} "
            f"model={summary['model_scored_rows']:,}",
            file=sys.stderr,
        )

    stability_result = stability(all_rows, args.top)
    args.out.joinpath("summary.json").write_text(
        json.dumps(
            {
                "config": {
                    "model": args.model,
                    "min_minutes": args.min_minutes,
                    "shrink_k": args.shrink_k,
                    "top_n": args.top,
                    "saves": {label: str(path) for label, path in saves.items()},
                },
                "saves": summaries,
                "stability": stability_result,
            },
            indent=2,
            ensure_ascii=False,
        ),
        encoding="utf-8",
    )

    field_rows = []
    for label, summary in summaries.items():
        for scope, values in summary["field_coverage"].items():
            for field, info in values.items():
                field_rows.append(
                    {
                        "save": label,
                        "scope": scope,
                        "field": field,
                        "non_null": info["non_null"],
                        "total": info["total"],
                        "pct": info["pct"],
                    }
                )
    write_csv(
        args.out / "field_coverage.csv",
        field_rows,
        ("save", "scope", "field", "non_null", "total", "pct"),
    )

    top_columns = (
        "player_uid",
        "player_name",
        "team_id",
        "multi_team",
        "other_team_minutes",
        "advanced_available",
        "club_name",
        "league_name",
        "group",
        "age",
        "ca",
        "pa",
        "club_reputation",
        "rep_tier",
        "minutes",
        "average_rating",
        "goals",
        "assists",
        "expected_goals",
        "expected_assists",
        "transfer_value",
        "transfer_value_state",
        "wage",
        "cost_total",
        "contract_end",
        "production",
        "coverage",
        "cost_pct",
        "surplus",
        "drivers",
    )
    for view in (
        "production_global",
        "production_tier",
        "surplus_global",
        "surplus_tier",
        "constrained_production",
        "constrained_surplus",
        "reputable_surplus",
    ):
        rows = []
        for label, summary in summaries.items():
            for row in summary["top"][view]:
                row = dict(row)
                row["save"] = label
                row["view"] = view
                row["drivers"] = "; ".join(
                    f"{driver['label']} {driver['percentile']}"
                    for driver in row["drivers"]
                )
                rows.append(row)
        write_csv(
            args.out / f"top_{view}.csv",
            rows,
            ("save", "view") + top_columns,
        )

    for view in ("by_group", "by_group_constrained", "by_group_surplus_constrained"):
        for group in GROUP_ORDER:
            if group == "OTHER":
                continue
            rows = []
            for label, summary in summaries.items():
                for row in summary.get(view, {}).get(group, []):
                    row = dict(row)
                    row["save"] = label
                    rows.append(row)
            if not rows:
                continue
            write_csv(
                args.out / f"top_group_{view}_{group}.csv",
                rows,
                ("save",) + top_columns,
            )

    sample_rows = []
    for label, summary in summaries.items():
        for row in summary["sample"]:
            row = dict(row)
            row["save"] = label
            sample_rows.append(row)
    write_csv(
        args.out / "sample_20.csv",
        sample_rows,
        ("save",) + SAMPLE_COLUMNS,
    )

    managed_rows = []
    for label, summary in summaries.items():
        for row in summary["managed_sample"]:
            row = dict(row)
            row["save"] = label
            managed_rows.append(row)
    write_csv(
        args.out / "sample_managed.csv",
        managed_rows,
        ("save",) + SAMPLE_COLUMNS,
    )

    write_report(
        args.out,
        summaries,
        stability_result,
        args.min_minutes,
        args.shrink_k,
        args.top,
        args.model,
    )
    print(f"wrote {args.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
