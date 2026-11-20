#!/usr/bin/env python3
"""Generate deterministic user-patch fixtures for the YM2413 EG AR/DR/KSR sweep."""
from __future__ import annotations

import argparse
import csv
import hashlib
import json
from pathlib import Path


def fixture_configs() -> list[tuple[int, int, int]]:
    """Return 38 representative AR/DR/KSR configurations without duplicates."""
    configs = {(10, dr, ksr) for dr in range(16) for ksr in (0, 1)}
    configs.update((ar, 5, ksr) for ar in (0, 5, 15) for ksr in (0, 1))
    return sorted(configs)


def generate_fixture(base_rows: list[list[str]], ar: int, dr: int, ksr: int) -> list[list[str]]:
    """Rewrite only user-patch bytes while preserving event timing and channel setup."""
    rows = [row.copy() for row in base_rows]
    current_address: int | None = None
    seen_patch_addresses: set[int] = set()

    for row in rows[1:]:
        if len(row) != 3:
            raise ValueError(f"invalid fixture row: {row!r}")
        call_index, port, value = row
        if port == "0":
            current_address = int(value)
            continue
        if port != "1" or current_address is None:
            raise ValueError(f"unexpected event ordering at call {call_index}: {row!r}")

        replacement: int | None = None
        if current_address == 0:
            replacement = 0x10 if ksr else 0x00  # modulator KR; EG remains 0
        elif current_address == 1:
            replacement = 0x30 if ksr else 0x20  # carrier KR + EG
        elif current_address in (4, 5):
            replacement = (ar << 4) | dr
        elif current_address in (6, 7):
            replacement = 0x47  # SL=4, RR=7

        if replacement is not None:
            row[2] = str(replacement)
            seen_patch_addresses.add(current_address)
        current_address = None

    expected = {0, 1, 4, 5, 6, 7}
    if seen_patch_addresses != expected:
        raise ValueError(
            f"fixture did not program all user-patch registers: "
            f"expected {sorted(expected)}, got {sorted(seen_patch_addresses)}"
        )
    return rows


def main() -> int:
    """Write the sweep fixtures and a SHA-256 manifest."""
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", required=True, type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    args = parser.parse_args()

    with args.base.open(encoding="utf-8", newline="") as stream:
        base_rows = list(csv.reader(stream))
    if not base_rows or base_rows[0] != ["# call_index", "port", "value"]:
        raise ValueError("unexpected base fixture header")

    args.output_dir.mkdir(parents=True, exist_ok=True)
    fixtures = []
    for ar, dr, ksr in fixture_configs():
        name = f"eg-rate-ar{ar:02d}-dr{dr:02d}-ksr{ksr}.csv"
        path = args.output_dir / name
        rows = generate_fixture(base_rows, ar, dr, ksr)
        with path.open("w", encoding="utf-8", newline="") as stream:
            csv.writer(stream, lineterminator="\n").writerows(rows)
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        fixtures.append({
            "fixture": name,
            "sha256": digest,
            "ar": ar,
            "dr": dr,
            "ksr": ksr,
            "sl": 4,
            "rr": 7,
            "fnum": 128,
            "blocks": [0, 2, 4, 6],
            "channels": [0, 1, 2, 3],
            "calls": 30000,
        })

    manifest = {
        "schema": "ym2413-eg-rate-sweep-v1",
        "base_fixture": args.base.name,
        "configuration_count": len(fixtures),
        "configurations": fixtures,
    }
    (args.output_dir / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(json.dumps({
        "configuration_count": len(fixtures),
        "output_dir": str(args.output_dir),
        "manifest": str(args.output_dir / "manifest.json"),
    }, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
