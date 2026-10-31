#!/usr/bin/env python3
"""Export the oracle's competition index to JSON for comparison with the Rust parser.

This is a development tool. It is not shipped with the app.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import fmsave

DEFAULT_SAVE = (
    Path.home()
    / "Library/Application Support/Sports Interactive/Football Manager 27/games"
    / "BB KING - Casciavit.fm"
)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("save", nargs="?", default=str(DEFAULT_SAVE))
    parser.add_argument("--output", default="/tmp/oracle_competitions.json")
    args = parser.parse_args()

    with fmsave.open(args.save, experimental_fm27=True) as save:
        competitions = [
            {
                "id": competition.id,
                "database_id": competition.database_id,
                "name": competition.name,
                "stage_ids": list(competition.stage_ids),
            }
            for competition in save.competitions()
        ]
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("w", encoding="utf-8") as handle:
        json.dump(competitions, handle, ensure_ascii=False)
    print(f"wrote {len(competitions)} competitions to {output}")


if __name__ == "__main__":
    main()
