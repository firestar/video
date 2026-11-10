#!/usr/bin/env python3
"""Run the real-input UI e2e suite against the deployed site.

Usage:
  python3 e2e-ui/run.py                # against https://video.synload.com
  E2E_UI_BASE=https://... python3 e2e-ui/run.py

Drives a real Chromium with genuine input (CDP key/mouse/touch/wheel events,
Drives a real Chromium with genuine input (CDP key/mouse/touch/wheel events,
real file input). Reuses a fixed pair of test accounts (registered once, then
logged in) and creates its own videos with real ffmpeg bytes; every video and
playlist it creates is deleted again in the finally block. Exit code 0 = all
passed (skips allowed), 1 = failures.

Prereqs: ffmpeg on PATH, `pip install playwright && playwright install chromium`.
"""
import importlib
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from harness import Ctx  # noqa: E402
from ui import UI  # noqa: E402

MODULES = [
    "test_01_upload",
    "test_02_studio",
    "test_03_playlists",
    "test_04_notifications",
    "test_05_shorts",
    "test_06_big_upload",
    "test_07_thumbnail",
    "test_08_subtitles",
    "test_09_embed",
    "test_10_age",
]


def main():
    ctx = Ctx()
    t0 = time.time()
    print(f"=== real-input UI e2e run {ctx.run_id} ===")
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
