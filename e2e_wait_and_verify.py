#!/usr/bin/env python3
"""Wait for the history fix to deploy (POST /api/history -> 204), then run the full e2e suite.

Run from the repo root:  python3 e2e_wait_and_verify.py
"""
import json, os, time, urllib.request, urllib.error, subprocess, sys

BASE = os.environ.get("E2E_BASE", "https://video.synload.com")
UA = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/120.0 Safari/537.36"


def req(method, path, token=None, body=None):
    data = json.dumps(body).encode() if body is not None else None
    r = urllib.request.Request(BASE + path, data=data, method=method)
    r.add_header("User-Agent", UA)
    if data:
        r.add_header("Content-Type", "application/json")
    if token:
        r.add_header("Authorization", f"Bearer {token}")
    try:
        with urllib.request.urlopen(r, timeout=15) as resp:
            txt = resp.read().decode()
            return resp.status, (json.loads(txt) if txt else None)
    except urllib.error.HTTPError as e:
        try:
            return e.code, json.loads(e.read().decode())
        except Exception:
            return e.code, None


def history_fixed():
    """True if POST /api/history returns 204 (the RETURNING fix is live)."""
    t = int(time.time() * 1000)
    s, me = req("POST", "/api/auth/register", body={
        "name": "H Probe", "email": f"hprobe-{t}@test.local",
        "password": "TestPass123!", "channelName": f"H Probe {t}"})
    if s != 200:
        return False
    tok = me["token"]
    s, v = req("POST", "/api/videos", tok, body={
        "title": "H Probe Vid", "objectKey": f"hprobe/{t}.mp4", "visibility": "public"})
    if s != 201:
        return False
    vid = v["video"]["id"]
    s, _ = req("POST", "/api/history", tok, body={"videoId": vid, "watchedUntilSecs": 0})
    req("DELETE", f"/api/videos/{vid}", tok)
    return s == 204


def main():
    print(f"base: {BASE}", flush=True)
    print("Waiting for the history fix to deploy (POST /api/history -> 204)...", flush=True)
    live = False
    for i in range(1, 121):  # up to ~20 min
        if history_fixed():
            print(f"HISTORY FIX LIVE after ~{i * 10}s", flush=True)
            live = True
            break
        print(f"[{time.strftime('%H:%M:%S')}] not live yet ({i})", flush=True)
        time.sleep(10)
    if not live:
        print("history fix NOT live after 1200s — running suite anyway", flush=True)

    print("\n=== Running full e2e suite ===", flush=True)
    r = subprocess.run([sys.executable, os.path.join(os.path.dirname(os.path.abspath(__file__)), "e2e", "run.py")])
    sys.exit(r.returncode)

if __name__ == "__main__":
    main()
