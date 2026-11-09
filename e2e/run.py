#!/usr/bin/env python3
"""Run the full API e2e suite against a live backend.

Usage:
  python3 e2e/run.py                 # against https://video.synload.com
  E2E_BASE=https://host python3 e2e/run.py
  E2E_SQL_CMD='ssh ... psql -tA' python3 e2e/run.py   # enable ready-video checks

Each test_*.py module is run in order; a module that raises is recorded as a
failure but does not stop the rest. Exits 0 only if there were no failures.
"""
import importlib
import os
import sys
import time
import traceback

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from harness import Ctx, BASE  # noqa: E402


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    modules = sorted(
        f for f in os.listdir(here)
        if f.startswith("test_") and f.endswith(".py")
    )
    print(f"=== synload API e2e suite ===")
    print(f"base: {BASE}")
    print(f"modules: {', '.join(m[:-3] for m in modules)}")
    print(f"E2E_SQL_CMD: {'set' if os.environ.get('E2E_SQL_CMD') else 'not set (ready-video checks skipped)'}")
    print()

    ctx = Ctx()
    ctx.reset_fixed_state()
    t0 = time.time()
    for mod_name in modules:
        label = mod_name[:-3]
        print(f"--- {label} ---")
        try:
            mod = importlib.import_module(mod_name[:-3])
            mod.run(ctx)
        except Exception:
            ctx.results.failed += 1
            ctx.results.failures.append((label, "module raised"))
            print(f"  ERROR in {label}:")
            traceback.print_exc()
        print()

    # best-effort cleanup of created videos
    try:
        ctx.cleanup()
    except Exception:
        pass

    r = ctx.results
    dt = time.time() - t0
    print("=" * 50)
    print(f"RESULT: {r.passed} passed, {r.failed} failed, {r.skipped} skipped  ({dt:.1f}s)")
    if r.failures:
        print("\nFailures:")
        for name, detail in r.failures:
            print(f"  - {name}: {detail}")
    print("=" * 50)
    sys.exit(1 if r.failed else 0)


if __name__ == "__main__":
    main()
