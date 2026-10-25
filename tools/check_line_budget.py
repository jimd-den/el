#!/usr/bin/env python3
"""File line-budget checker. No source file may exceed MAX (200) physical lines."""
import sys
from pathlib import Path

MAX = 200
ROOT = Path(__file__).resolve().parents[1]
EXEMPT_MARKER = "line-budget: exempt"
BASELINE = ROOT / "tools" / "line-budget-baseline.txt"

COUNT_EXTS = {".rs", ".py", ".toml", ".sh", ".yaml", ".yml"}

def is_exempt(path: Path) -> bool:
    try:
        head = path.read_text().splitlines()[:2]
    except Exception:
        return False
    return any(EXEMPT_MARKER in line for line in head)

def count_lines(path: Path) -> int:
    return len(path.read_text().splitlines())

def main() -> int:
    violations = []
    # tracked source files
    for p in ROOT.rglob("*"):
        if not p.is_file():
            continue
        if "target" in p.parts:
            continue
        if p.suffix not in COUNT_EXTS and p.name not in ("Makefile",):
            continue
        if is_exempt(p):
            continue
        n = count_lines(p)
        if n > MAX:
            violations.append((p.relative_to(ROOT), n))
    if violations:
        print("LINE BUDGET VIOLATIONS:")
        for p, n in violations:
            print(f"  {p}: {n} lines (max {MAX})")
        return 1
    print("line budget: ok")
    return 0

if __name__ == "__main__":
    sys.exit(main())
