#!/usr/bin/env python3
"""Compare the Rust competition index with the fmsave oracle export."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


def load(path: str) -> list[dict]:
    with Path(path).open(encoding="utf-8") as handle:
        return json.load(handle)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("oracle")
    parser.add_argument("rust")
    parser.add_argument("--max-report", type=int, default=20)
    args = parser.parse_args()

    oracle = {row["id"]: row for row in load(args.oracle)}
    rust = {row["id"]: row for row in load(args.rust)}
    missing = sorted(set(oracle) - set(rust))
    extra = sorted(set(rust) - set(oracle))
    print(f"oracle={len(oracle)} rust={len(rust)} missing={len(missing)} extra={len(extra)}")

    mismatches = 0
    for competition_id in sorted(set(oracle) & set(rust)):
        expected = oracle[competition_id]
        actual = rust[competition_id]
        differences = [
            f"{key}: {expected.get(key)!r} != {actual.get(key)!r}"
            for key in ("database_id", "name", "stage_ids")
            if expected.get(key) != actual.get(key)
        ]
        if differences:
            mismatches += 1
            if mismatches <= args.max_report:
                print(f"\nid {competition_id}")
                for difference in differences:
                    print("  ", difference)
    print(f"\ncompetitions with differences: {mismatches}")


if __name__ == "__main__":
    main()
