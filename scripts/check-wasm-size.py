#!/usr/bin/env python3
"""Assert the counter-spa gzipped wasm stays under the size budget.

Picks the most recently built `*_bg.wasm` from `examples/counter-spa/dist`
(produced by `trunk build --release`), gzips it, and compares the compressed
size against a budget in KiB. Exits non-zero if it exceeds the budget, so it
acts as a CI gate against DX additions silently bloating the shipped binary.

Usage: python3 scripts/check-wasm-size.py [BUDGET_KB]
"""
import gzip
import os
import pathlib
import sys

REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
DIST_DIR = REPO_ROOT / "examples" / "counter-spa" / "dist"

DEFAULT_BUDGET_KB = 60


def main() -> int:
    budget_kb = float(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_BUDGET_KB

    candidates = sorted(DIST_DIR.glob("*_bg.wasm"), key=os.path.getmtime, reverse=True)
    if not candidates:
        print(f"error: no *_bg.wasm found under {DIST_DIR}", file=sys.stderr)
        print("run `make size` (or `trunk build --release`) first", file=sys.stderr)
        return 2

    wasm = candidates[0]
    data = wasm.read_bytes()
    gzip_bytes = len(gzip.compress(data))
    size_kb = gzip_bytes / 1024.0

    print(f"gzipped wasm: {size_kb:.1f} kB (budget {budget_kb:.0f} kB) -> {wasm.name}")
    if size_kb > budget_kb:
        print(
            f"FAIL: gzipped wasm {size_kb:.1f} kB exceeds budget {budget_kb:.0f} kB\n"
            "The DX additions are bloating the shipped binary. Investigate, or "
            "raise BUDGET_KB in the Makefile deliberately.",
            file=sys.stderr,
        )
        return 1
    print("OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())