#!/usr/bin/env python3
"""Export clubs, finances, facilities and stadiums from the fmsave oracle.

This is a development tool. It is not shipped with the app.
"""

from __future__ import annotations

import argparse
import dataclasses
import json
import warnings
from datetime import date
from enum import IntEnum
from pathlib import Path

import fmsave

DEFAULT_SAVE = (
    Path.home()
    / "Library/Application Support/Sports Interactive/Football Manager 27/games"
    / "BB KING - Parma Calcio.fm"
)


def jsonable(value):
    if isinstance(value, date):
        return value.isoformat()
    if isinstance(value, IntEnum):
        return int(value)
    if dataclasses.is_dataclass(value):
        return {field.name: jsonable(getattr(value, field.name)) for field in dataclasses.fields(value)}
    if isinstance(value, dict):
        return {str(key): jsonable(item) for key, item in value.items()}
    if isinstance(value, (list, tuple, set)):
        return [jsonable(item) for item in value]
    return value


def club_row(club: fmsave.Club) -> dict:
    return {
        "uid": club.uid,
        "unique_id": club.unique_id,
        "name": club.name,
        "short_name": club.short_name,
        "nation_id": club.nation_id,
        "fa_nation_id": club.fa_nation_id,
        "city_id": club.city_id,
        "gender": None if club.gender is None else club.gender.raw,
        "reputation": club.reputation,
        "last_league_position": club.last_league_position,
        "parent_club_uid": club.parent_club_uid,
        "parent_club_name": club.parent_club_name,
        "teams": [team.team_id for team in club.teams],
    }


def finance_row(row: fmsave.FinanceMonth) -> dict:
    return {
        "club_uid": row.club_uid,
        "month": row.month.isoformat(),
        "balance": row.balance,
        "transfer_budget_allocated": row.transfer_budget_allocated,
        "transfer_budget_remaining": row.transfer_budget_remaining,
        "wage_budget_weekly": row.wage_budget_weekly,
        "wage_payroll_weekly": row.wage_payroll_weekly,
        "income_excluding_transfers": row.income_excluding_transfers,
        "net_transfers": row.net_transfers,
        "wage_bill": row.wage_bill,
        "net": row.net,
        "expenditure_excluding_transfers": row.expenditure_excluding_transfers,
        "total_income": row.total_income,
        "total_expenditure": row.total_expenditure,
    }


def facility_row(row: fmsave.ClubFacilities) -> dict:
    return {
        "club_uid": row.club_uid,
        "corporate_facilities": row.corporate_facilities.raw,
    }


def stadium_row(row: fmsave.Stadium) -> dict:
    return {
        "uid": row.uid,
        "name": row.name,
        "all_seater_capacity": row.all_seater_capacity,
        "expansion_capacity": row.expansion_capacity,
        "capacity": row.capacity,
        "owner_club_uid": row.owner_club_uid,
        "pitch_length_dm": row.pitch_length_dm,
        "pitch_width_dm": row.pitch_width_dm,
        "pitch_min_length_dm": row.pitch_min_length_dm,
        "pitch_min_width_dm": row.pitch_min_width_dm,
        "pitch_max_length_dm": row.pitch_max_length_dm,
        "pitch_max_width_dm": row.pitch_max_width_dm,
        "built_date": jsonable(row.built_date),
        "rebuilt_date": jsonable(row.rebuilt_date),
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("save", nargs="?", default=str(DEFAULT_SAVE))
    parser.add_argument("--output", required=True)
    args = parser.parse_args()

    warnings.filterwarnings("ignore")
    save = fmsave.open(args.save, experimental_fm27=True)
    payload = {
        "clubs": [club_row(club) for club in save.clubs()],
        "club_finances": [finance_row(row) for row in save.finances()],
        "club_facilities": [facility_row(row) for row in save.facilities()],
        "stadiums": [stadium_row(row) for row in save.stadiums()],
    }
    Path(args.output).write_text(json.dumps(payload), encoding="utf-8")
    print(
        f"clubs={len(payload['clubs'])} finance={len(payload['club_finances'])} "
        f"facilities={len(payload['club_facilities'])} stadiums={len(payload['stadiums'])}"
    )


if __name__ == "__main__":
    main()
