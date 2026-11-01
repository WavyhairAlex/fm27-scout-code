#!/usr/bin/env python3
"""Compare one player in a before/after FM27 save pair.

The script uses the fm27scout CLI to export each save's players to a temporary
JSON file, then prints the CA/PA, personality, traits and every changed raw
attribute for the requested uid.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CLI = REPO_ROOT / "target" / "release" / "fm27scout"


def load_player(cli: Path, save: Path, uid: int) -> dict:
    with tempfile.NamedTemporaryFile(suffix=".json", delete=False) as handle:
        output = Path(handle.name)
    try:
        subprocess.run(
            [
                str(cli),
                "players",
                str(save),
                "--limit",
                "50000",
                "--format",
                "json",
                "--output",
                str(output),
            ],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        players = json.loads(output.read_text())
    finally:
        try:
            output.unlink()
        except FileNotFoundError:
            pass
    for player in players:
        if player["uid"] == uid:
            return player
    raise SystemExit(f"player uid {uid} was not found in {save}")


def named_traits(player: dict) -> list[str]:
    return [trait["name"] or f"bit {trait['bit']}" for trait in player.get("traits", [])]


def print_player(label: str, player: dict) -> None:
    print(f"== {label}")
    print(f"name: {player.get('name')}")
    print(f"CA/PA: {player['ca']} / {player.get('pa')}")
    print(f"value: {player.get('transfer_value')}")
    print(f"personality: {player.get('personality')}")
    print(f"traits: {player.get('trait_bits')} {named_traits(player)}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", required=True, type=Path)
    parser.add_argument("--after", required=True, type=Path)
    parser.add_argument("--uid", required=True, type=int)
    parser.add_argument("--fm27scout", type=Path, default=DEFAULT_CLI)
    parser.add_argument(
        "--all-attributes",
        action="store_true",
        help="print unchanged attributes too, not only the changed ones",
    )
    args = parser.parse_args()

    if not args.fm27scout.exists():
        parser.error(f"fm27scout binary not found: {args.fm27scout}")
    baseline = load_player(args.fm27scout, args.baseline, args.uid)
    after = load_player(args.fm27scout, args.after, args.uid)

    print_player("baseline", baseline)
    print_player("after", after)

    before_attrs = baseline["raw_attributes"]
    after_attrs = after["raw_attributes"]
    print("\nraw attributes:")
    print(f"{'key':24s} {'before':>6s} {'after':>6s} {'delta':>6s}")
    changed = 0
    for key in before_attrs:
        before = before_attrs[key]
        value = after_attrs[key]
        if args.all_attributes or before != value:
            changed += 1
            print(f"{key:24s} {before:6d} {value:6d} {value - before:+6d}")
    print(
        f"\nsum raw attributes: {sum(before_attrs.values())} -> "
        f"{sum(after_attrs.values())} ({changed} shown)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
