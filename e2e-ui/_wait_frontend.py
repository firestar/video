#!/usr/bin/env python3
"""Poll the deployed site until the new frontend (shorts testids) is live.
Exit 0 when live, 1 after ~15 min."""
import re
import sys
import time
import urllib.request

BASE = "https://video.synload.com"
UA = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/120.0 Safari/537.36"
MARKER = "shorts-feed"


def get(url):
    r = urllib.request.Request(url, headers={"User-Agent": UA})
    with urllib.request.urlopen(r, timeout=20) as resp:
        return resp.read().decode(errors="replace")


def live():
    html = get(BASE + "/shorts")
    chunks = set(re.findall(r'"/_next/static/[^"]+\.js"', html))
    for c in chunks:
        try:
            if MARKER in get(BASE + c.strip('"')):
                return True
        except Exception:
            pass
    return False


def main():
    for i in range(1, 61):  # up to ~15 min
        try:
            if live():
                print(f"NEW FRONTEND LIVE after ~{i * 15}s", flush=True)
                sys.exit(0)
        except Exception as e:
            print(f"[{time.strftime('%H:%M:%S')}] probe error: {e}", flush=True)
        print(f"[{time.strftime('%H:%M:%S')}] not live yet ({i})", flush=True)
        time.sleep(15)
    print("new frontend NOT live after 900s", flush=True)
    sys.exit(1)


if __name__ == "__main__":
    main()
