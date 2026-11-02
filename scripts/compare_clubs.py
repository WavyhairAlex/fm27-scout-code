#!/usr/bin/env python3
"""Compare Rust club output with the fmsave oracle export.

Usage:
    python3 scripts/compare_clubs.py oracle_clubs.json rust_clubs.json
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

CLUB_FIELDS = (
    "name",
    "short_name",
    "nation_id",
    "fa_nation_id",
    "city_id",
    "gender",
    "reputation",
    "last_league_position",
    "parent_club_uid",
    "parent_club_name",
    "unique_id",
)

FINANCE_FIELDS = (
    "balance",
    "transfer_budget_allocated",
    "transfer_budget_remaining",
    "wage_budget_weekly",
    "wage_payroll_weekly",
    "income_excluding_transfers",
    "net_transfers",
    "wage_bill",
    "net",
    "expenditure_excluding_transfers",
    "total_income",
    "total_expenditure",
)

STADIUM_FIELDS = (
    "name",
    "all_seater_capacity",
    "expansion_capacity",
    "capacity",
    "owner_club_uid",
    "pitch_length_dm",
    "pitch_width_dm",
    "pitch_min_length_dm",
    "pitch_min_width_dm",
    "pitch_max_length_dm",
    "pitch_max_width_dm",
    "built_date",
    "rebuilt_date",
)


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def compare_maps(
    label: str,
    oracle: dict,
    rust: dict,
    fields: tuple[str, ...],
    mismatches: list[str],
) -> None:
    missing = sorted(set(oracle) - set(rust))
    extra = sorted(set(rust) - set(oracle))
    print(f"{label}: oracle={len(oracle)} rust={len(rust)} missing={len(missing)} extra={len(extra)}")
    if missing:
        mismatches.append(f"{label}: missing keys {missing[:5]}")
    if extra:
        mismatches.append(f"{label}: extra keys {extra[:5]}")
    for key in sorted(set(oracle) & set(rust)):
        for field in fields:
            if oracle[key].get(field) != rust[key].get(field):
                mismatches.append(
                    f"{label} {key}: {field} oracle={oracle[key].get(field)!r} "
                    f"rust={rust[key].get(field)!r}"
                )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("oracle")
    parser.add_argument("rust")
    args = parser.parse_args()
    oracle = load(Path(args.oracle))
    rust = load(Path(args.rust))

    mismatches: list[str] = []

    oracle_clubs = {row["uid"]: row for row in oracle["clubs"]}
    rust_clubs = {row["uid"]: row for row in rust["clubs"]}
    compare_maps("clubs", oracle_clubs, rust_clubs, CLUB_FIELDS, mismatches)
    for key in sorted(set(oracle_clubs) & set(rust_clubs)):
        if sorted(oracle_clubs[key]["teams"]) != sorted(rust_clubs[key]["teams"]):
            mismatches.append(f"clubs {key}: teams differ")
            if len(mismatches) > 20:
                break

    oracle_finance = {
        (row["club_uid"], row["month"]): row for row in oracle["club_finances"]
    }
    rust_finance = {
        (row["club_uid"], row["month"]): row for row in rust["club_finances"]
    }
    compare_maps(
        "club_finances", oracle_finance, rust_finance, FINANCE_FIELDS, mismatches
    )

    oracle_facilities = {
        row["club_uid"]: row["corporate_facilities"]
        for row in oracle["club_facilities"]
        if row["corporate_facilities"]
    }
    rust_facilities = {
        uid: row["corporate_facilities"]
        for uid, row in rust_clubs.items()
        if row.get("corporate_facilities") is not None
    }
    print(
        f"club_facilities: oracle={len(oracle_facilities)} rust={len(rust_facilities)}"
    )
    for uid in sorted(set(oracle_facilities) | set(rust_facilities)):
        if oracle_facilities.get(uid) != rust_facilities.get(uid):
            mismatches.append(
                f"club_facilities {uid}: oracle={oracle_facilities.get(uid)!r} "
                f"rust={rust_facilities.get(uid)!r}"
            )

    oracle_stadiums = {row["uid"]: row for row in oracle["stadiums"]}
    rust_stadiums = {row["uid"]: row for row in rust["stadiums"]}
    compare_maps("stadiums", oracle_stadiums, rust_stadiums, STADIUM_FIELDS, mismatches)

    print(f"mismatches: {len(mismatches)}")
    for message in mismatches[:30]:
        print("  " + message)
    raise SystemExit(1 if mismatches else 0)


if __name__ == "__main__":
    main()
