#!/usr/bin/env python3
"""Run ONLY test_07_thumbnail (focused verification of the thumbnail fix)."""
import importlib
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from harness import Ctx  # noqa: E402
from ui import UI  # noqa: E402

MODULES = ["test_07_thumbnail"]


def main():
    ctx = Ctx()
    t0 = time.time()
    print(f"=== standalone test_07 run {ctx.run_id} ===")
    ui = UI(ctx.run_id)
    ctx.ui = ui
    try:
        for name in MODULES:
            mod = importlib.import_module(name)
            print(f"\n--- {name} ---")
            mod.run(ctx)
    except Exception as e:
        import traceback
        traceback.print_exc()
        ctx.results.failed += 1
        ctx.results.failures.append(("unhandled exception", repr(e)))
    finally:
        ui.close()
        ctx.cleanup()

    r = ctx.results
    secs = int(time.time() - t0)
    print(f"\n=== SUMMARY: {r.passed} passed, {r.failed} failed, "
          f"{r.skipped} skipped ({secs}s) ===")
    for name, detail in r.failures:
        print(f"  FAIL  {name}  {detail}")
    print(f"screenshots: {ui.shot_dir}")
    sys.exit(1 if r.failed else 0)


if __name__ == "__main__":
    main()
