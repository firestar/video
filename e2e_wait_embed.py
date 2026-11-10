#!/usr/bin/env python3
"""Wait for the frontend re-deploy (aria-label fix) to go live, then run both
e2e suites.

Deploy marker: the Next.js build id embedded in the homepage script tags
(`type="<buildId>-text/javascript"`) changes when a new frontend build is
served. `OLD_BUILD_ID` is the build currently live; the new deploy is live
once the id differs. No accounts are created by the probe.
"""
import os
import re
import subprocess
import sys
import time
import urllib.request

BASE = os.environ.get("E2E_BASE", "https://video.synload.com")
UA = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/120.0 Safari/537.36"
OLD_BUILD_ID = "9f791ad01770b67720d139bd"
REPO = os.path.dirname(os.path.abspath(__file__))


def current_build_id():
    r = urllib.request.Request(BASE + "/", method="GET")
    r.add_header("User-Agent", UA)
    try:
        with urllib.request.urlopen(r, timeout=15) as resp:
            html = resp.read(4000).decode("utf-8", "replace")
        m = re.search(r'type="([0-9a-f]{24})-text/javascript"', html)
        return m.group(1) if m else None
    except Exception:
        return None


def main():
    print(f"base: {BASE}", flush=True)
    print(f"waiting for a frontend build different from {OLD_BUILD_ID}...",
          flush=True)
    live = False
    for i in range(1, 51):  # up to ~25 min
        bid = current_build_id()
        if bid and bid != OLD_BUILD_ID:
            print(f"NEW FRONTEND LIVE after ~{i * 30}s (build {bid})", flush=True)
            live = True
            break
        print(f"[{time.strftime('%H:%M:%S')}] build still {bid} ({i})", flush=True)
        time.sleep(30)
    if not live:
        print("new build NOT live after 25 min — running suites anyway", flush=True)

    print("\n=== API e2e suite ===", flush=True)
    r1 = subprocess.run([sys.executable, os.path.join(REPO, "e2e", "run.py")])
    print("\n=== UI e2e suite ===", flush=True)
    r2 = subprocess.run([sys.executable, os.path.join(REPO, "e2e-ui", "run.py")])
    sys.exit(1 if (r1.returncode or r2.returncode) else 0)


if __name__ == "__main__":
    main()
